pub trait XmssOperation<T> {
    fn operation(&self, height: usize, lhs: T, rhs: T) -> T;
}

pub struct XmssPath<T>(Vec<(T, bool)>);

impl<T> XmssPath<T> {
    pub fn advance<F>(self, item: T, f: &F) -> T
    where
        F: XmssOperation<T>,
    {
        self.0
            .into_iter()
            .enumerate()
            .fold(item, |item, (i, (other, reverse))| match reverse {
                false => f.operation(i, item, other),
                true => f.operation(i, other, item),
            })
    }
}

#[derive(Clone)]
pub struct XmssTree<T>(pub Vec<T>);

impl<T> XmssTree<T> {
    pub fn path<F>(self, item: T, f: &F) -> (T, XmssPath<T>)
    where
        F: XmssOperation<T>,
        T: Eq,
    {
        let _ = (item, f);
        unimplemented!()
    }

    pub fn collapse<F>(self, f: &F) -> T
    where
        F: XmssOperation<T>,
    {
        use core::mem;

        let XmssTree(data) = self;

        assert!(!data.is_empty());
        let height = mem::size_of::<usize>() * 8 - ((data.len() - 1).leading_zeros() as usize);
        let mut data = (0..height).fold(data, |data, index| {
            let capacity = data.len() / 2 + 1;
            let (state, mut new) = data.into_iter().fold(
                (None, Vec::with_capacity(capacity)),
                |(accumulator, mut new), item| match accumulator {
                    None => (Some(item), new),
                    Some(left) => {
                        new.push(f.operation(index, left, item));
                        (None, new)
                    },
                },
            );
            match state {
                None => new,
                Some(item) => {
                    new.push(item);
                    new
                },
            }
        });
        assert!(data.len() == 1);
        data.pop().unwrap()
    }
}

#[cfg(test)]
#[test]
fn test_xmss_tree_collapse() {
    impl XmssOperation<usize> for () {
        fn operation(&self, height: usize, lhs: usize, rhs: usize) -> usize {
            let _ = height;
            lhs + rhs
        }
    }

    for &n in &[67, 21, 17, 34, 16, 32, 64] {
        let x = XmssTree((0..n).collect()).collapse(&());
        assert_eq!(x, n * (n - 1) / 2);
    }
}
