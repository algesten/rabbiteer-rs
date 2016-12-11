
#[macro_export]
macro_rules! errln(
    ($($arg:tt)*) => { {
        use std::io::Write;
        writeln!(&mut ::std::io::stderr(), $($arg)*).expect("failed printing to stderr");
    } }
);


#[macro_export]
macro_rules! rbterr(
    ($($arg:tt)*) => {{
        return Err(::error::RbtError::Message(format!($($arg)*)));
    }}
);
