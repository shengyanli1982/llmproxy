/// Option 扩展 trait，提供额外的辅助方法
pub trait IsNoneOr<T> {
    /// 检查 Option 是否为 None 或者满足指定条件
    ///
    /// # 参数
    /// * `predicate` - 一个函数，用于检查 Option 内部值是否满足条件
    ///
    /// # 返回值
    /// 如果 Option 为 None 或者内部值满足条件，则返回 true，否则返回 false
    fn is_none_or<F>(&self, predicate: F) -> bool
    where
        F: FnOnce(&T) -> bool;
}

impl<T> IsNoneOr<T> for Option<T> {
    fn is_none_or<F>(&self, predicate: F) -> bool
    where
        F: FnOnce(&T) -> bool,
    {
        match self {
            None => true,
            Some(value) => predicate(value),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_is_none_or() {
        let none: Option<String> = None;
        assert!(none.is_none_or(|s| s.is_empty()));

        let empty = Some(String::new());
        assert!(empty.is_none_or(|s| s.is_empty()));

        let not_empty = Some("hello".to_string());
        assert!(!not_empty.is_none_or(|s| s.is_empty()));
    }
}
