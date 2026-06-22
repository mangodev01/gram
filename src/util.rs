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

pub fn shorten(max_len_before_shortening: i32, s: &str) -> String {
    if max_len_before_shortening < 0 {
        return s.to_string();
    }
    let max_len = max_len_before_shortening as usize;
    if s.chars().count() > max_len {
        s.chars().take(max_len).collect::<String>() + "..."
    } else {
        s.to_string()
    }
}



