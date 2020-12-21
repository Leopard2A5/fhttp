use crate::errors::Result;
use crate::request::body::Body;

pub trait HasBody {
    fn body(&self) -> Result<Body>;
}
