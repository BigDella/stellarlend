/// Parallel blockchain indexer with worker pools for concurrent block processing
use crate::cache::CacheService;
use crate::config::Config;
use crate::error::{IndexerError, IndexerResult};
use crate::models::{CreateEvent, EventUpdate, UpdateType};
use crate::parser::EventParser;
use crate::repository::EventRepository;
use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Duration, Instant};
use tracing::{debug, error, info, warn};

/// Checkpoint for restart on failure
#[derive(Clone, Debug)]
struct IndexingCheckpoint {
    contract_address: String,
    last_processed_block: u64,
    timestamp: chrono::DateTime<chrono::Utc>,
}

/// Worker pool metrics
#[derive(Clone, Debug)]
pub struct WorkerPoolMetrics {
    pub active_workers: usize,
    pub total_blocks_processed: u64,
    pub blocks_per_minute: f64,
    pub avg_block_time_ms: f64,
    pub failed_blocks: u64,
    pub reorg_count: u64,
}

/// Parallel indexer service with worker pools
pub struct ParallelIndexerService {
    provider: Arc<Provider<Ws>>,
    parser: Arc<RwLock<EventParser>>,
    repository: EventRepository,
    cache: Arc<RwLock<CacheService>>,
    config: Config,
    is_running: Arc<RwLock<bool>>,
    checkpoints: Arc<RwLock<HashMap<String, IndexingCheckpoint>>>,
    metrics: Arc<RwLock<WorkerPoolMetrics>>,
    worker_semaphore: Arc<Semaphore>,
}

impl ParallelIndexerService {
    /// Create a new parallel indexer service
    pub async fn new(
        config: Config,
        repository: EventRepository,
        cache: CacheService,
    ) -> IndexerResult<Self> {
        let provider = Provider::<Ws>::connect(&config.blockchain.ws_url)
            .await
            .map_err(|e| IndexerError::Rpc(format!("Failed to connect: {}", e)))?;

        info!("Parallel indexer connected to {}", config.blockchain.ws_url);

        let worker_concurrency = config.indexer.worker_concurrency.unwrap_or(4);

        Ok(Self {
            provider: Arc::new(provider),
            parser: Arc::new(RwLock::new(EventParser::new())),
            repository,
            cache: Arc::new(RwLock::new(cache)),
            config,
            is_running: Arc::new(RwLock::new(false)),
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(WorkerPoolMetrics {
                active_workers: 0,
                total_blocks_processed: 0,
                blocks_per_minute: 0.0,
                avg_block_time_ms: 0.0,
                failed_blocks: 0,
                reorg_count: 0,
            })),
            worker_semaphore: Arc::new(Semaphore::new(worker_concurrency)),
        })
    }

    /// Register a contract for parallel indexing
    pub async fn register_contract(
        &self,
        contract_address: &str,
        abi_json: &str,
        start_block: u64,
    ) -> IndexerResult<()> {
        let mut parser = self.parser.write().await;
        parser.register_contract(contract_address, abi_json)?;
        drop(parser);

        self.repository
            .get_or_create_metadata(contract_address, start_block)
            .await?;

        // Initialize checkpoint
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.insert(
            contract_address.to_string(),
            IndexingCheckpoint {
                contract_address: contract_address.to_string(),
                last_processed_block: start_block.saturating_sub(1),
                timestamp: chrono::Utc::now(),
            },
        );

        info!(
            "Registered contract {} for parallel indexing from block {}",
            contract_address, start_block
        );
        Ok(())
    }

    /// Start parallel indexing with worker pools
    pub async fn start(&self) -> IndexerResult<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            warn!("Parallel indexer already running");
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        info!("Starting parallel indexer with worker pools");

        let start_time = Instant::now();
        let mut blocks_processed_at_start = 0u64;

        loop {
            if !*self.is_running.read().await {
                info!("Parallel indexer stopped");
                break;
            }

            let metadata_list = self.repository.get_active_metadata().await?;
            if metadata_list.is_empty() {
                sleep(Duration::from_secs(self.config.indexer.poll_interval)).await;
                continue;
            }

            let current_block = self.get_current_block().await?;

            // Process each contract in parallel
            let mut handles: Vec<JoinHandle<IndexerResult<()>>> = Vec::new();

            for metadata in metadata_list {
                let from_block = (metadata.last_indexed_block + 1) as u64;
                let to_block = current_block.saturating_sub(self.config.indexer.confirmations);

                if from_block > to_block {
                    continue;
                }

                // Partition block range for parallel processing
                let partitions = self.partition_block_range(from_block, to_block);

                for (partition_start, partition_end) in partitions {
                    let handle = self.spawn_worker(
                        metadata.contract_address.clone(),
                        partition_start,
                        partition_end,
                    );
                    handles.push(handle);
                }
            }

            // Wait for all workers to complete
            for handle in handles {
                if let Err(e) = handle.await {
                    error!("Worker task failed: {}", e);
                }
            }

            // Update metrics
            self.update_metrics(start_time, blocks_processed_at_start).await;

            // Save checkpoints
            self.save_checkpoints().await?;

            sleep(Duration::from_secs(self.config.indexer.poll_interval)).await;
        }

        Ok(())
    }

    /// Partition block range into chunks for parallel processing
    fn partition_block_range(&self, from_block: u64, to_block: u64) -> Vec<(u64, u64)> {
        let total_blocks = to_block.saturating_sub(from_block) + 1;
        let partition_size = self.config.indexer.batch_size;
        let mut partitions = Vec::new();

        let mut current = from_block;
        while current <= to_block {
            let end = std::cmp::min(current + partition_size - 1, to_block);
            partitions.push((current, end));
            current = end + 1;
        }

        debug!(
            "Partitioned {} blocks into {} partitions",
            total_blocks,
            partitions.len()
        );

        partitions
    }

    /// Spawn a worker to process a block range
    fn spawn_worker(
        &self,
        contract_address: String,
        from_block: u64,
        to_block: u64,
    ) -> JoinHandle<IndexerResult<()>> {
        let provider = Arc::clone(&self.provider);
        let parser = Arc::clone(&self.parser);
        let repository = self.repository.clone();
        let cache = Arc::clone(&self.cache);
        let config = self.config.clone();
        let metrics = Arc::clone(&self.metrics);
        let semaphore = Arc::clone(&self.worker_semaphore);
        let checkpoints = Arc::clone(&self.checkpoints);

        tokio::spawn(async move {
            // Acquire semaphore permit (limits concurrency)
            let _permit = semaphore.acquire().await.unwrap();

            // Increment active workers
            {
                let mut m = metrics.write().await;
                m.active_workers += 1;
            }

            let start_time = Instant::now();

            let result = Self::process_block_range_worker(
                provider,
                parser,
                repository,
                cache,
                &contract_address,
                from_block,
                to_block,
                &config,
            )
            .await;

            let elapsed = start_time.elapsed().as_millis() as f64;

            // Update metrics
            {
                let mut m = metrics.write().await;
                m.active_workers = m.active_workers.saturating_sub(1);

                match &result {
                    Ok(count) => {
                        m.total_blocks_processed += (to_block - from_block + 1);
                        let blocks = (to_block - from_block + 1) as f64;
                        m.avg_block_time_ms = (m.avg_block_time_ms + elapsed / blocks) / 2.0;
                        
                        info!(
                            "Worker processed {} blocks ({}-{}) with {} events in {:.2}ms",
                            blocks, from_block, to_block, count, elapsed
                        );
                    }
                    Err(e) => {
                        m.failed_blocks += (to_block - from_block + 1);
                        error!(
                            "Worker failed to process blocks {}-{}: {}",
                            from_block, to_block, e
                        );
                    }
                }
            }

            // Update checkpoint on success
            if result.is_ok() {
                let mut checkpoints_guard = checkpoints.write().await;
                if let Some(checkpoint) = checkpoints_guard.get_mut(&contract_address) {
                    if to_block > checkpoint.last_processed_block {
                        checkpoint.last_processed_block = to_block;
                        checkpoint.timestamp = chrono::Utc::now();
                    }
                }
            }

            result.map(|_| ())
        })
    }

    /// Worker function to process a block range
    async fn process_block_range_worker(
        provider: Arc<Provider<Ws>>,
        parser: Arc<RwLock<EventParser>>,
        repository: EventRepository,
        cache: Arc<RwLock<CacheService>>,
        contract_address: &str,
        from_block: u64,
        to_block: u64,
        config: &Config,
    ) -> IndexerResult<usize> {
        let address: Address = contract_address
            .parse()
            .map_err(|e| IndexerError::EventParsing(format!("Invalid address: {}", e)))?;

        let filter = Filter::new()
            .address(address)
            .from_block(from_block)
            .to_block(to_block);

        // Fetch logs with retry
        let logs = Self::fetch_logs_with_retry(&provider, &filter, config.indexer.max_retries)
            .await?;

        if logs.is_empty() {
            return Ok(0);
        }

        // Parse logs
        let parser_guard = parser.read().await;
        let mut events = Vec::new();

        for log in logs {
            if let Some(event) = parser_guard.parse_log(&log)? {
                events.push(event);
            }
        }
        drop(parser_guard);

        let event_count = events.len();

        // Batch insert
        if !events.is_empty() {
            repository.create_events_batch(events).await?;

            // Invalidate cache
            let mut cache_guard = cache.write().await;
            cache_guard.invalidate_queries().await?;
            cache_guard.set_latest_block(to_block).await?;
        }

        Ok(event_count)
    }

    /// Fetch logs with retry logic
    async fn fetch_logs_with_retry(
        provider: &Provider<Ws>,
        filter: &Filter,
        max_retries: u32,
    ) -> IndexerResult<Vec<Log>> {
        for attempt in 0..=max_retries {
            match provider.get_logs(filter).await {
                Ok(logs) => return Ok(logs),
                Err(e) if attempt < max_retries => {
                    warn!("Log fetch attempt {} failed, retrying: {}", attempt + 1, e);
                    sleep(Duration::from_millis(1000 * (2u64.pow(attempt)))).await;
                }
                Err(e) => {
                    return Err(IndexerError::Rpc(format!("Failed to fetch logs: {}", e)));
                }
            }
        }
        unreachable!()
    }

    /// Handle blockchain reorganization with parallel workers
    pub async fn handle_reorg(&self, reorg_block: u64) -> IndexerResult<()> {
        warn!("Handling reorg from block {}", reorg_block);

        // Graceful worker shutdown
        self.stop().await;

        // Delete affected events
        let deleted = self.repository.delete_events_from_block(reorg_block).await?;
        info!("Deleted {} events due to reorg", deleted);

        // Update checkpoints
        let mut checkpoints = self.checkpoints.write().await;
        for checkpoint in checkpoints.values_mut() {
            if checkpoint.last_processed_block >= reorg_block {
                checkpoint.last_processed_block = reorg_block.saturating_sub(1);
                checkpoint.timestamp = chrono::Utc::now();
            }
        }

        // Update metrics
        {
            let mut m = self.metrics.write().await;
            m.reorg_count += 1;
        }

        // Restart indexing
        *self.is_running.write().await = true;

        Ok(())
    }

    /// Update performance metrics
    async fn update_metrics(&self, start_time: Instant, initial_blocks: u64) {
        let mut m = self.metrics.write().await;
        let elapsed_minutes = start_time.elapsed().as_secs_f64() / 60.0;
        
        if elapsed_minutes > 0.0 {
            let blocks_since_start = m.total_blocks_processed.saturating_sub(initial_blocks);
            m.blocks_per_minute = blocks_since_start as f64 / elapsed_minutes;
        }
    }

    /// Save checkpoints to persistent storage
    async fn save_checkpoints(&self) -> IndexerResult<()> {
        let checkpoints = self.checkpoints.read().await;
        
        for checkpoint in checkpoints.values() {
            self.repository
                .update_metadata(
                    &checkpoint.contract_address,
                    checkpoint.last_processed_block,
                )
                .await?;
        }

        Ok(())
    }

    /// Get current block number
    async fn get_current_block(&self) -> IndexerResult<u64> {
        self.provider
            .get_block_number()
            .await
            .map(|n| n.as_u64())
            .map_err(|e| IndexerError::Rpc(format!("Failed to get block number: {}", e)))
    }

    /// Stop the indexer (graceful shutdown)
    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        info!("Stopping parallel indexer (graceful shutdown)");

        // Wait for active workers to complete
        loop {
            let active = self.metrics.read().await.active_workers;
            if active == 0 {
                break;
            }
            debug!("Waiting for {} active workers to complete", active);
            sleep(Duration::from_millis(500)).await;
        }

        info!("All workers stopped");
    }

    /// Get indexing metrics
    pub async fn get_metrics(&self) -> WorkerPoolMetrics {
        self.metrics.read().await.clone()
    }

    /// Get indexing lag (blocks behind current)
    pub async fn get_indexing_lag(&self) -> IndexerResult<u64> {
        let current_block = self.get_current_block().await?;
        let checkpoints = self.checkpoints.read().await;
        
        let min_indexed = checkpoints
            .values()
            .map(|c| c.last_processed_block)
            .min()
            .unwrap_or(0);

        Ok(current_block.saturating_sub(min_indexed))
    }
}
