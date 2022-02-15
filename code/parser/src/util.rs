use std::fmt::Debug;

pub(crate) trait ResultIter<T, E>
where Self: Iterator<Item = Result<T, E>> {
    fn try_collect(self) -> Result<Vec<T>, Vec<E>>;
}

impl<T, E, I> ResultIter<T, E> for I where I: Iterator<Item = Result<T, E>>, E: Debug {
    fn try_collect(self) -> Result<Vec<T>, Vec<E>> {
        let mut items: Vec<_> = self.collect();
        if items.iter().any(|e| e.is_err()) {
            Err(items
                .drain(..)
                .filter_map(|item| match item {
                    Err(e) => Some(e),
                    _ => None
                })
                .collect())
        } else {
            Ok(items.drain(..).map(|item| item.unwrap()).collect())
        }
    }
}
