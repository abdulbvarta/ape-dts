use std::{
    fs::{self, File},
    io::Read,
    str::FromStr,
};

use anyhow::bail;

use crate::error::Error;

use super::{
    config_enums::{ConflictPolicyEnum, DbType, ExtractType, ParallelType, SinkType},
    data_marker_config::DataMarkerConfig,
    extractor_config::{BasicExtractorConfig, ExtractorConfig},
    filter_config::FilterConfig,
    ini_loader::IniLoader,
    parallelizer_config::ParallelizerConfig,
    pipeline_config::PipelineConfig,
    processor_config::ProcessorConfig,
    resumer_config::ResumerConfig,
    router_config::RouterConfig,
    runtime_config::RuntimeConfig,
    s3_config::S3Config,
    sinker_config::{BasicSinkerConfig, SinkerConfig},
};

#[derive(Clone)]
pub struct TaskConfig {
    pub extractor_basic: BasicExtractorConfig,
    pub extractor: ExtractorConfig,
    pub sinker_basic: BasicSinkerConfig,
    pub sinker: SinkerConfig,
    pub runtime: RuntimeConfig,
    pub parallelizer: ParallelizerConfig,
    pub pipeline: PipelineConfig,
    pub filter: FilterConfig,
    pub router: RouterConfig,
    pub resumer: ResumerConfig,
    pub data_marker: Option<DataMarkerConfig>,
    pub processor: Option<ProcessorConfig>,
}

// sections
const EXTRACTOR: &str = "extractor";
const SINKER: &str = "sinker";
const PIPELINE: &str = "pipeline";
const PARALLELIZER: &str = "parallelizer";
const RUNTIME: &str = "runtime";
const FILTER: &str = "filter";
const ROUTER: &str = "router";
const RESUMER: &str = "resumer";
const DATA_MARKER: &str = "data_marker";
const PROCESSOR: &str = "processor";
// keys
const CHECK_LOG_DIR: &str = "check_log_dir";
const DB_TYPE: &str = "db_type";
const URL: &str = "url";
const BATCH_SIZE: &str = "batch_size";
const SAMPLE_INTERVAL: &str = "sample_interval";
const HEARTBEAT_INTERVAL_SECS: &str = "heartbeat_interval_secs";
const KEEPALIVE_INTERVAL_SECS: &str = "keepalive_interval_secs";
const HEARTBEAT_TB: &str = "heartbeat_tb";
const APP_NAME: &str = "app_name";
const REVERSE: &str = "reverse";
const REPL_PORT: &str = "repl_port";
// default values
const APE_DTS: &str = "APE_DTS";
const ASTRISK: &str = "*";

impl TaskConfig {
    pub fn new(task_config_file: &str) -> anyhow::Result<Self> {
        let loader = IniLoader::new(task_config_file);

        let (extractor_basic, extractor) = Self::load_extractor_config(&loader)?;
        let (sinker_basic, sinker) = Self::load_sinker_config(&loader)?;
        Ok(Self {
            extractor_basic,
            extractor,
            parallelizer: Self::load_parallelizer_config(&loader)?,
            pipeline: Self::load_pipeline_config(&loader),
            sinker_basic,
            sinker,
            runtime: Self::load_runtime_config(&loader)?,
            filter: Self::load_filter_config(&loader)?,
            router: Self::load_router_config(&loader)?,
            resumer: Self::load_resumer_config(&loader)?,
            data_marker: Self::load_data_marker_config(&loader)?,
            processor: Self::load_processor_config(&loader)?,
        })
    }

    fn load_extractor_config(
        loader: &IniLoader,
    ) -> anyhow::Result<(BasicExtractorConfig, ExtractorConfig)> {
        let db_type_str: String = loader.get_required(EXTRACTOR, DB_TYPE);
        let extract_type_str: String = loader.get_required(EXTRACTOR, "extract_type");
        let db_type = DbType::from_str(&db_type_str)?;
        let extract_type = ExtractType::from_str(&extract_type_str)?;

        let url: String = loader.get_optional(EXTRACTOR, URL);
        let heartbeat_interval_secs: u64 =
            loader.get_with_default(EXTRACTOR, HEARTBEAT_INTERVAL_SECS, 10);
        let keepalive_interval_secs: u64 =
            loader.get_with_default(EXTRACTOR, KEEPALIVE_INTERVAL_SECS, 10);
        let heartbeat_tb = loader.get_optional(EXTRACTOR, HEARTBEAT_TB);

        let basic = BasicExtractorConfig {
            db_type: db_type.clone(),
            extract_type: extract_type.clone(),
            url: url.clone(),
        };

        let not_supported_err =
            Error::ConfigError(format!("extract type: {} not supported", extract_type));

        let sinker = match db_type {
            DbType::Mysql => match extract_type {
                ExtractType::Snapshot => ExtractorConfig::MysqlSnapshot {
                    url,
                    db: String::new(),
                    tb: String::new(),
                    sample_interval: loader.get_with_default(EXTRACTOR, SAMPLE_INTERVAL, 1),
                },

                ExtractType::Cdc => ExtractorConfig::MysqlCdc {
                    url,
                    binlog_filename: loader.get_optional(EXTRACTOR, "binlog_filename"),
                    binlog_position: loader.get_optional(EXTRACTOR, "binlog_position"),
                    server_id: loader.get_required(EXTRACTOR, "server_id"),
                    gtid_enabled: loader.get_optional(EXTRACTOR, "gtid_enabled"),
                    gtid_set: loader.get_optional(EXTRACTOR, "gtid_set"),
                    heartbeat_interval_secs,
                    heartbeat_tb,
                    start_time_utc: loader.get_optional(EXTRACTOR, "start_time_utc"),
                    end_time_utc: loader.get_optional(EXTRACTOR, "end_time_utc"),
                },

                ExtractType::CheckLog => ExtractorConfig::MysqlCheck {
                    url,
                    check_log_dir: loader.get_required(EXTRACTOR, CHECK_LOG_DIR),
                    batch_size: loader.get_with_default(EXTRACTOR, BATCH_SIZE, 200),
                },

                ExtractType::Struct => ExtractorConfig::MysqlStruct {
                    url,
                    db: String::new(),
                },

                ExtractType::FoxlakeS3 => {
                    let s3_config = S3Config {
                        bucket: loader.get_optional(EXTRACTOR, "s3_bucket"),
                        access_key: loader.get_optional(EXTRACTOR, "s3_access_key"),
                        secret_key: loader.get_optional(EXTRACTOR, "s3_secret_key"),
                        region: loader.get_optional(EXTRACTOR, "s3_region"),
                        endpoint: loader.get_optional(EXTRACTOR, "s3_endpoint"),
                        root_dir: loader.get_optional(EXTRACTOR, "s3_root_dir"),
                        root_url: loader.get_optional(EXTRACTOR, "s3_root_url"),
                    };
                    ExtractorConfig::FoxlakeS3 {
                        url,
                        schema: String::new(),
                        tb: String::new(),
                        s3_config,
                    }
                }

                _ => bail! {not_supported_err},
            },

            DbType::Pg => match extract_type {
                ExtractType::Snapshot => ExtractorConfig::PgSnapshot {
                    url,
                    schema: String::new(),
                    tb: String::new(),
                    sample_interval: loader.get_with_default(EXTRACTOR, SAMPLE_INTERVAL, 1),
                },

                ExtractType::Cdc => ExtractorConfig::PgCdc {
                    url,
                    slot_name: loader.get_required(EXTRACTOR, "slot_name"),
                    pub_name: loader.get_optional(EXTRACTOR, "pub_name"),
                    start_lsn: loader.get_optional(EXTRACTOR, "start_lsn"),
                    keepalive_interval_secs,
                    heartbeat_interval_secs,
                    heartbeat_tb,
                    ddl_command_tb: loader.get_optional(EXTRACTOR, "ddl_command_tb"),
                    start_time_utc: loader.get_optional(EXTRACTOR, "start_time_utc"),
                    end_time_utc: loader.get_optional(EXTRACTOR, "end_time_utc"),
                },

                ExtractType::CheckLog => ExtractorConfig::PgCheck {
                    url,
                    check_log_dir: loader.get_required(EXTRACTOR, CHECK_LOG_DIR),
                    batch_size: loader.get_with_default(EXTRACTOR, BATCH_SIZE, 200),
                },

                ExtractType::Struct => ExtractorConfig::PgStruct {
                    url,
                    schema: String::new(),
                },

                _ => bail! { not_supported_err },
            },

            DbType::Mongo => {
                let app_name: String =
                    loader.get_with_default(EXTRACTOR, APP_NAME, APE_DTS.to_string());
                match extract_type {
                    ExtractType::Snapshot => ExtractorConfig::MongoSnapshot {
                        url,
                        app_name,
                        db: String::new(),
                        tb: String::new(),
                    },

                    ExtractType::Cdc => ExtractorConfig::MongoCdc {
                        url,
                        app_name,
                        resume_token: loader.get_optional(EXTRACTOR, "resume_token"),
                        start_timestamp: loader.get_optional(EXTRACTOR, "start_timestamp"),
                        source: loader.get_optional(EXTRACTOR, "source"),
                        heartbeat_interval_secs,
                        heartbeat_tb,
                    },

                    ExtractType::CheckLog => ExtractorConfig::MongoCheck {
                        url,
                        app_name,
                        check_log_dir: loader.get_required(EXTRACTOR, CHECK_LOG_DIR),
                        batch_size: loader.get_with_default(EXTRACTOR, BATCH_SIZE, 200),
                    },

                    _ => bail! { not_supported_err },
                }
            }

            DbType::Redis => match extract_type {
                ExtractType::Snapshot => {
                    let repl_port = loader.get_with_default(EXTRACTOR, REPL_PORT, 10008);
                    ExtractorConfig::RedisSnapshot { url, repl_port }
                }

                ExtractType::SnapshotFile => ExtractorConfig::RedisSnapshotFile {
                    file_path: loader.get_required(EXTRACTOR, "file_path"),
                },

                ExtractType::Scan => ExtractorConfig::RedisScan {
                    url,
                    statistic_type: loader.get_required(EXTRACTOR, "statistic_type"),
                    scan_count: loader.get_with_default(EXTRACTOR, "scan_count", 1000),
                },

                ExtractType::Cdc => {
                    let repl_port = loader.get_with_default(EXTRACTOR, REPL_PORT, 10008);
                    ExtractorConfig::RedisCdc {
                        url,
                        repl_port,
                        repl_id: loader.get_optional(EXTRACTOR, "repl_id"),
                        repl_offset: loader.get_optional(EXTRACTOR, "repl_offset"),
                        keepalive_interval_secs,
                        heartbeat_interval_secs,
                        heartbeat_key: loader.get_optional(EXTRACTOR, "heartbeat_key"),
                        now_db_id: loader.get_optional(EXTRACTOR, "now_db_id"),
                    }
                }

                ExtractType::Reshard => {
                    let to_node_ids = loader.get_required(EXTRACTOR, "to_node_ids");
                    ExtractorConfig::RedisReshard { url, to_node_ids }
                }

                _ => bail! { not_supported_err },
            },

            DbType::Kafka => ExtractorConfig::Kafka {
                url,
                group: loader.get_required(EXTRACTOR, "group"),
                topic: loader.get_required(EXTRACTOR, "topic"),
                partition: loader.get_optional(EXTRACTOR, "partition"),
                offset: loader.get_optional(EXTRACTOR, "offset"),
                ack_interval_secs: loader.get_optional(EXTRACTOR, "ack_interval_secs"),
            },

            db_type => {
                bail! {Error::ConfigError(format!(
                    "extractor db type: {} not supported",
                    db_type
                ))}
            }
        };
        Ok((basic, sinker))
    }

    fn load_sinker_config(loader: &IniLoader) -> anyhow::Result<(BasicSinkerConfig, SinkerConfig)> {
        let db_type_str: String = loader.get_required(SINKER, DB_TYPE);
        let sink_type_str = loader.get_with_default(SINKER, "sink_type", "write".to_string());
        let db_type = DbType::from_str(&db_type_str)?;
        let sink_type = SinkType::from_str(&sink_type_str)?;

        let url: String = loader.get_optional(SINKER, URL);
        let batch_size: usize = loader.get_with_default(SINKER, BATCH_SIZE, 200);

        let basic = BasicSinkerConfig {
            db_type: db_type.clone(),
            url: url.clone(),
            batch_size,
        };

        let conflict_policy_str: String = loader.get_optional(SINKER, "conflict_policy");
        let conflict_policy = ConflictPolicyEnum::from_str(&conflict_policy_str)?;

        let not_supported_err =
            Error::ConfigError(format!("sinker db type: {} not supported", db_type));

        let sinker = match db_type {
            DbType::Mysql => match sink_type {
                SinkType::Write => SinkerConfig::Mysql { url, batch_size },

                SinkType::Check => SinkerConfig::MysqlCheck {
                    url,
                    batch_size,
                    check_log_dir: loader.get_optional(SINKER, CHECK_LOG_DIR),
                },

                SinkType::Struct => SinkerConfig::MysqlStruct {
                    url,
                    conflict_policy,
                },

                SinkType::Sql => SinkerConfig::Sql {
                    reverse: loader.get_optional(SINKER, REVERSE),
                },

                _ => bail! { not_supported_err },
            },

            DbType::Pg => match sink_type {
                SinkType::Write => SinkerConfig::Pg { url, batch_size },

                SinkType::Check => SinkerConfig::PgCheck {
                    url,
                    batch_size,
                    check_log_dir: loader.get_optional(SINKER, CHECK_LOG_DIR),
                },

                SinkType::Struct => SinkerConfig::PgStruct {
                    url,
                    conflict_policy,
                },

                SinkType::Sql => SinkerConfig::Sql {
                    reverse: loader.get_optional(SINKER, REVERSE),
                },

                _ => bail! { not_supported_err },
            },

            DbType::Mongo => {
                let app_name: String =
                    loader.get_with_default(SINKER, APP_NAME, APE_DTS.to_string());
                match sink_type {
                    SinkType::Write => SinkerConfig::Mongo {
                        url,
                        app_name,
                        batch_size,
                    },

                    SinkType::Check => SinkerConfig::MongoCheck {
                        url,
                        app_name,
                        batch_size,
                        check_log_dir: loader.get_optional(SINKER, CHECK_LOG_DIR),
                    },

                    _ => bail! { not_supported_err },
                }
            }

            DbType::Kafka => SinkerConfig::Kafka {
                url,
                batch_size,
                ack_timeout_secs: loader.get_required(SINKER, "ack_timeout_secs"),
                required_acks: loader.get_required(SINKER, "required_acks"),
            },

            DbType::Redis => match sink_type {
                SinkType::Write => SinkerConfig::Redis {
                    url,
                    batch_size,
                    method: loader.get_optional(SINKER, "method"),
                    is_cluster: loader.get_optional(SINKER, "is_cluster"),
                },

                SinkType::Statistic => SinkerConfig::RedisStatistic {
                    statistic_type: loader.get_required(SINKER, "statistic_type"),
                    data_size_threshold: loader.get_optional(SINKER, "data_size_threshold"),
                    freq_threshold: loader.get_optional(SINKER, "freq_threshold"),
                    statistic_log_dir: loader.get_optional(SINKER, "statistic_log_dir"),
                },

                SinkType::Dummy => SinkerConfig::Dummy,

                _ => bail! { not_supported_err },
            },

            DbType::StarRocks => SinkerConfig::Starrocks {
                url,
                batch_size,
                stream_load_url: loader.get_optional(SINKER, "stream_load_url"),
            },

            DbType::Foxlake => {
                let s3_config = S3Config {
                    bucket: loader.get_optional(SINKER, "s3_bucket"),
                    access_key: loader.get_optional(SINKER, "s3_access_key"),
                    secret_key: loader.get_optional(SINKER, "s3_secret_key"),
                    region: loader.get_optional(SINKER, "s3_region"),
                    endpoint: loader.get_optional(SINKER, "s3_endpoint"),
                    root_dir: loader.get_optional(SINKER, "s3_root_dir"),
                    root_url: loader.get_optional(SINKER, "s3_root_url"),
                };

                match sink_type {
                    SinkType::Write => SinkerConfig::Foxlake {
                        url,
                        batch_size,
                        batch_memory_mb: loader.get_optional(SINKER, "batch_memory_mb"),
                        s3_config,
                        engine: loader.get_optional(SINKER, "engine"),
                    },

                    SinkType::Struct => SinkerConfig::FoxlakeStruct {
                        url,
                        conflict_policy,
                        engine: loader.get_optional(SINKER, "engine"),
                    },

                    SinkType::Push => SinkerConfig::FoxlakePush {
                        url,
                        batch_size,
                        batch_memory_mb: loader.get_optional(SINKER, "batch_memory_mb"),
                        s3_config,
                    },

                    SinkType::Merge => SinkerConfig::FoxlakeMerge {
                        url,
                        batch_size,
                        s3_config,
                    },

                    _ => bail! { not_supported_err },
                }
            }
        };
        Ok((basic, sinker))
    }

    fn load_parallelizer_config(loader: &IniLoader) -> anyhow::Result<ParallelizerConfig> {
        let parallel_type_str =
            loader.get_with_default(PARALLELIZER, "parallel_type", "serial".to_string());
        Ok(ParallelizerConfig {
            parallel_size: loader.get_with_default(PARALLELIZER, "parallel_size", 1),
            parallel_type: ParallelType::from_str(&parallel_type_str)?,
        })
    }

    fn load_pipeline_config(loader: &IniLoader) -> PipelineConfig {
        let mut config = PipelineConfig {
            buffer_size: loader.get_with_default(PIPELINE, "buffer_size", 16000),
            checkpoint_interval_secs: loader.get_with_default(
                PIPELINE,
                "checkpoint_interval_secs",
                10,
            ),
            batch_sink_interval_secs: loader.get_optional(PIPELINE, "batch_sink_interval_secs"),
            counter_time_window_secs: loader.get_optional(PIPELINE, "counter_time_window_secs"),
            counter_max_sub_count: loader.get_with_default(PIPELINE, "counter_max_sub_count", 1000),
            max_rps: loader.get_optional(PIPELINE, "max_rps"),
            buffer_memory_mb: loader.get_optional(PIPELINE, "buffer_memory_mb"),
        };

        if config.counter_time_window_secs == 0 {
            config.counter_time_window_secs = config.checkpoint_interval_secs;
        }
        config
    }

    fn load_runtime_config(loader: &IniLoader) -> anyhow::Result<RuntimeConfig> {
        Ok(RuntimeConfig {
            log_level: loader.get_with_default(RUNTIME, "log_level", "info".to_string()),
            log_dir: loader.get_with_default(RUNTIME, "log_dir", "./logs".to_string()),
            log4rs_file: loader.get_with_default(
                RUNTIME,
                "log4rs_file",
                "./log4rs.yaml".to_string(),
            ),
        })
    }

    fn load_filter_config(loader: &IniLoader) -> anyhow::Result<FilterConfig> {
        Ok(FilterConfig {
            do_schemas: loader.get_optional(FILTER, "do_dbs"),
            ignore_schemas: loader.get_optional(FILTER, "ignore_dbs"),
            do_tbs: loader.get_optional(FILTER, "do_tbs"),
            ignore_tbs: loader.get_optional(FILTER, "ignore_tbs"),
            do_events: loader.get_optional(FILTER, "do_events"),
            do_ddls: loader.get_optional(FILTER, "do_ddls"),
            do_structures: loader.get_with_default(FILTER, "do_structures", ASTRISK.to_string()),
            ignore_cmds: loader.get_optional(FILTER, "ignore_cmds"),
        })
    }

    fn load_router_config(loader: &IniLoader) -> anyhow::Result<RouterConfig> {
        Ok(RouterConfig::Rdb {
            schema_map: loader.get_optional(ROUTER, "db_map"),
            tb_map: loader.get_optional(ROUTER, "tb_map"),
            col_map: loader.get_optional(ROUTER, "col_map"),
            topic_map: loader.get_optional(ROUTER, "topic_map"),
        })
    }

    fn load_resumer_config(loader: &IniLoader) -> anyhow::Result<ResumerConfig> {
        let mut resume_log_dir: String = loader.get_optional(RESUMER, "resume_log_dir");
        if resume_log_dir.is_empty() {
            resume_log_dir = loader.get_with_default(RUNTIME, "log_dir", "./logs".to_string());
        }

        Ok(ResumerConfig {
            resume_config_file: loader.get_optional(RESUMER, "resume_config_file"),
            resume_from_log: loader.get_optional(RESUMER, "resume_from_log"),
            resume_log_dir,
        })
    }

    fn load_data_marker_config(loader: &IniLoader) -> anyhow::Result<Option<DataMarkerConfig>> {
        if !loader.ini.sections().contains(&DATA_MARKER.to_string()) {
            return Ok(None);
        }

        Ok(Some(DataMarkerConfig {
            topo_name: loader.get_required(DATA_MARKER, "topo_name"),
            topo_nodes: loader.get_optional(DATA_MARKER, "topo_nodes"),
            src_node: loader.get_required(DATA_MARKER, "src_node"),
            dst_node: loader.get_required(DATA_MARKER, "dst_node"),
            do_nodes: loader.get_required(DATA_MARKER, "do_nodes"),
            ignore_nodes: loader.get_optional(DATA_MARKER, "ignore_nodes"),
            marker: loader.get_required(DATA_MARKER, "marker"),
        }))
    }

    fn load_processor_config(loader: &IniLoader) -> anyhow::Result<Option<ProcessorConfig>> {
        if !loader.ini.sections().contains(&PROCESSOR.to_string()) {
            return Ok(None);
        }

        let lua_code_file = loader.get_optional(PROCESSOR, "lua_code_file");
        let mut lua_code = String::new();

        if fs::metadata(&lua_code_file).is_ok() {
            let mut file = File::open(&lua_code_file).expect("failed to open lua code file");
            file.read_to_string(&mut lua_code)
                .expect("failed to read lua code file");
        }

        Ok(Some(ProcessorConfig {
            lua_code_file,
            lua_code,
        }))
    }
}
