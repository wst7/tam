use log::LevelFilter;
use log4rs::{
    append::rolling_file::{
        policy::compound::{
            roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
        },
        RollingFileAppender,
    },
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};

use crate::config;

pub fn init_logging() {
    let tam_config = config::get_config();
    // 设置滚动策略：文件超过 10 MB 就轮转
    let size_trigger = SizeTrigger::new(10 * 1024 * 1024); // 10MB

    let roller = FixedWindowRoller::builder()
        .base(1)
        .build(&tam_config.logfile_pattern, 5)
        .expect("Failed to create roller");

    let compound_policy = CompoundPolicy::new(Box::new(size_trigger), Box::new(roller));

    // 创建 rolling_file appender
    let rolling_appender = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S.%f)} [{l}] {t} - {m}{n}",
        )))
        .build(&tam_config.logfile_path, Box::new(compound_policy))
        .expect("Failed to create rolling file appender");

    // 构建配置
    let config = Config::builder()
        .appender(Appender::builder().build("rolling_file", Box::new(rolling_appender)))
        .build(
            Root::builder()
                .appender("rolling_file")
                .build(LevelFilter::Info),
        )
        .expect("Failed to build log config");

    log4rs::init_config(config).expect("Failed to initialize log4rs");
}
