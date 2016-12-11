

#[macro_export]
macro_rules! errln(
    ($($arg:tt)*) => { {
        writeln!(&mut ::std::io::stderr(), $($arg)*).expect("failed printing to stderr");
    } }
);


#[macro_export]
macro_rules! exitln(
    ($($arg:tt)*) => { {
        errln!($($arg)*);
        process::exit(1);
    } }
);
