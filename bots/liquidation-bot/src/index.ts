import { LiquidationBotService } from './liquidationBot.service';

const bot = new LiquidationBotService();

bot.start();

process.on('SIGINT', () => {
  console.log('\nShutting down liquidation bot...');
  bot.stop();
  process.exit(0);
});

process.on('SIGTERM', () => {
  console.log('\nShutting down liquidation bot...');
  bot.stop();
  process.exit(0);
});

process.on('unhandledRejection', (reason) => {
  console.error('Unhandled rejection:', reason);
});

process.on('uncaughtException', (error) => {
  console.error('Uncaught exception:', error);
  process.exit(1);
});
