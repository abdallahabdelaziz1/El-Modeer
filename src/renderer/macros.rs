#[macro_export]
macro_rules! float_to_byte_string {
    ($x:expr, $unit:expr) => {
        match Byte::from_unit($x, $unit) {
            Ok(b) => b.get_appropriate_unit(false).to_string().replace(" ", ""),
            Err(_) => String::from("Err"),
        }
    };
}

#[macro_export]
macro_rules! convert_result_to_string {
    ($x:expr) => {
        match $x {
            Ok(_r) => String::from("Signal Sent."),
            Err(e) => convert_error_to_string!(e),
        }
    };
}

#[macro_export]
macro_rules! convert_error_to_string {
    ($x:expr) => {
        match $x {
            ProcessError::NoSuchProcess { .. } => String::from("No Such Process"),
            ProcessError::ZombieProcess { .. } => String::from("Zombie Process"),
            ProcessError::AccessDenied { .. } => String::from("Access Denied"),
            _ => String::from("Unknown error"),
        }
    };
}