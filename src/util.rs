pub macro info($colors:expr, $($arg:tt)*) {
    if $colors {
        color_print::cprintln!(
            "<cyan>{}</cyan>",
            format!($($arg)*)
        );
    } else {
        println!($($arg)*);
    }
}

pub macro error($colors:expr, $($arg:tt)*) {
    if $colors {
        color_print::ceprintln!(
            "<red>{}</red>",
            format!($($arg)*)
        );
    } else {
        eprintln!($($arg)*);
    }
}

pub macro success($colors:expr, $($arg:tt)*) {
    if $colors {
        color_print::cprintln!(
            "<green>{}</green>",
            format!($($arg)*)
        );
    } else {
        println!($($arg)*);
    }
}
