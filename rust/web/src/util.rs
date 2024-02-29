pub trait OptString: Sized {
    fn if_some(self) -> Option<Self>;
}

impl OptString for String {
    fn if_some(self) -> Option<Self> {
        if self.is_empty() {
            return None;
        } else {
            return Some(self);
        }
    }
}

impl OptString for &String {
    fn if_some(self) -> Option<Self> {
        if self.is_empty() {
            return None;
        } else {
            return Some(self);
        }
    }
}
