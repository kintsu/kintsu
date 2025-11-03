use crate::checks::*;

crate::rule! {
    SelfRef in Form @ Warn: Unsafe; ""
}

impl Check for SelfRef {}
