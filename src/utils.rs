
#[macro_export]
macro_rules! print_success {
    ($msg:expr) => {{
      use colored::Colorize;
      println!("{} {}", "SUCCESS".on_green().bold(), $msg);
    }};
    ($fmt:expr, $($arg:tt)*) => {{
      use colored::Colorize;
      println!("{} {}", "SUCCESS".on_green().bold(), format!($fmt, $($arg)*));
    }};
}

#[macro_export]
macro_rules! print_error {
    ($msg:expr) => {{
      use colored::Colorize;
      println!("{} {}", "ERROR".on_red().bold(), $msg);
    }};
    ($fmt:expr, $($arg:tt)*) => {{
      use colored::Colorize;
      println!("{} {}", "ERROR".on_red().bold(), format!($fmt, $($arg)*));
    }};
}

#[macro_export]
macro_rules! print_info {
    ($msg:expr) => {{
      use colored::Colorize;
      println!("{} {}", "INFO".on_blue().bold(), $msg);
    }};
    ($fmt:expr, $($arg:tt)*) => {{
      use colored::Colorize;
      println!("{} {}", "INFO".on_blue().bold(), format!($fmt, $($arg)*));
    }};
}
