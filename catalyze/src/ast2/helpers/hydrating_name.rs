use std::borrow::Borrow;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone)]
pub struct HydratingName(pub *const str);

impl Eq for HydratingName {}

impl Borrow<str> for HydratingName {
    fn borrow(&self) -> &str {
        unsafe { &*self.0 }
    }
}

impl PartialEq<HydratingName> for HydratingName {
    fn eq(&self, other: &Self) -> bool {
        unsafe { &*self.0 }.eq(unsafe { &*other.0 })
    }
}

impl Hash for HydratingName {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { &*self.0 }.hash(state)
    }
}
