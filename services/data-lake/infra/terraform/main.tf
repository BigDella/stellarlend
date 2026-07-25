variable "project" {
  type        = string
  description = "Project / name prefix"
  default     = "stellarlend"
}

variable "environment" {
  type    = string
  default = "dev"
}

variable "aws_region" {
  type    = string
  default = "us-east-1"
}

variable "raw_retention_days" {
  type    = number
  default = 90
}

provider "aws" {
  region = var.aws_region
}

resource "aws_s3_bucket" "raw_lake" {
  bucket = "${var.project}-${var.environment}-raw-blockchain"
}

resource "aws_s3_bucket_versioning" "raw_lake" {
  bucket = aws_s3_bucket.raw_lake.id
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_s3_bucket_lifecycle_configuration" "raw_retention" {
  bucket = aws_s3_bucket.raw_lake.id

  rule {
    id     = "expire-raw-after-retention"
    status = "Enabled"

    filter {
      prefix = "raw/"
    }

    expiration {
      days = var.raw_retention_days
    }
  }

  rule {
    id     = "keep-aggregates-indefinitely"
    status = "Enabled"

    filter {
      prefix = "agg/"
    }

    transition {
      days          = 30
      storage_class = "STANDARD_IA"
    }
  }
}

resource "aws_s3_bucket_public_access_block" "raw_lake" {
  bucket                  = aws_s3_bucket.raw_lake.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

resource "aws_glue_catalog_database" "lake" {
  name = "${var.project}_${var.environment}_lake"
}

resource "aws_glue_catalog_table" "raw_transactions" {
  name          = "raw_transactions"
  database_name = aws_glue_catalog_database.lake.name
  table_type    = "EXTERNAL_TABLE"

  parameters = {
    classification = "parquet"
    EXTERNAL       = "TRUE"
  }

  storage_descriptor {
    location      = "s3://${aws_s3_bucket.raw_lake.bucket}/raw/"
    input_format  = "org.apache.hadoop.hive.ql.io.parquet.MapredParquetInputFormat"
    output_format = "org.apache.hadoop.hive.ql.io.parquet.MapredParquetOutputFormat"

    ser_de_info {
      name                  = "parquet"
      serialization_library = "org.apache.hadoop.hive.ql.io.parquet.serde.ParquetHiveSerDe"
    }

    columns {
      name = "ledger"
      type = "bigint"
    }
    columns {
      name = "tx_hash"
      type = "string"
    }
    columns {
      name = "contract_id"
      type = "string"
    }
    columns {
      name = "event_type"
      type = "string"
    }
    columns {
      name = "event_index"
      type = "int"
    }
    columns {
      name = "block_timestamp"
      type = "bigint"
    }
    columns {
      name = "user_address"
      type = "string"
    }
    columns {
      name = "asset_address"
      type = "string"
    }
    columns {
      name = "amount"
      type = "string"
    }
    columns {
      name = "payload_json"
      type = "string"
    }
    columns {
      name = "schema_version"
      type = "int"
    }
  }

  partition_keys {
    name = "date"
    type = "string"
  }

  partition_keys {
    name = "event_type"
    type = "string"
  }
}

resource "aws_iam_role" "lake_reader" {
  name = "${var.project}-${var.environment}-lake-reader"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "athena.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "lake_reader" {
  name = "${var.project}-${var.environment}-lake-reader"
  role = aws_iam_role.lake_reader.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = ["s3:GetObject", "s3:ListBucket"]
        Resource = [
          aws_s3_bucket.raw_lake.arn,
          "${aws_s3_bucket.raw_lake.arn}/*"
        ]
      },
      {
        Effect   = "Allow"
        Action   = ["glue:GetDatabase", "glue:GetTable", "glue:GetPartitions"]
        Resource = ["*"]
      }
    ]
  })
}

resource "aws_iam_role" "lake_writer" {
  name = "${var.project}-${var.environment}-lake-writer"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect = "Allow"
      Principal = {
        Service = "ecs-tasks.amazonaws.com"
      }
      Action = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "lake_writer" {
  name = "${var.project}-${var.environment}-lake-writer"
  role = aws_iam_role.lake_writer.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = ["s3:PutObject", "s3:AbortMultipartUpload", "s3:ListBucket"]
        Resource = [
          aws_s3_bucket.raw_lake.arn,
          "${aws_s3_bucket.raw_lake.arn}/*"
        ]
      }
    ]
  })
}

output "bucket_name" {
  value = aws_s3_bucket.raw_lake.bucket
}

output "glue_database" {
  value = aws_glue_catalog_database.lake.name
}

output "reader_role_arn" {
  value = aws_iam_role.lake_reader.arn
}

output "writer_role_arn" {
  value = aws_iam_role.lake_writer.arn
}
