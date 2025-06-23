// tests/config.rs

// 将 tests/config_tests 目录下的模块引入
mod config_tests {
    #[cfg(test)]
    mod admin;
    #[cfg(test)]
    mod common;
    #[cfg(test)]
    mod file;
    #[cfg(test)]
    mod forward;
    #[cfg(test)]
    mod group;
    #[cfg(test)]
    mod routing;
    #[cfg(test)]
    mod upstream;
    #[cfg(test)]
    mod validation;
}
