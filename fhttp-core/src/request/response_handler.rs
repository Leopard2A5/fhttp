use regex::Regex;

use crate::{Result, FhttpError, Request, ResponseHandler};

pub trait RequestResponseHandlerExt {
    fn response_handler(&self) -> Result<Option<ResponseHandler>>;
}

impl RequestResponseHandlerExt for Request {
    fn response_handler(&self) -> Result<Option<ResponseHandler>> {
        lazy_static! {
            static ref RE_RESPONSE_HANDLER: Regex = Regex::new(r"(?sm)>\s*\{%(.*)%}").unwrap();
        };

        if let Some(captures) = RE_RESPONSE_HANDLER.captures(&self.text) {
            if let Some(group) = captures.get(1) {
                let group = group.as_str().trim();
                let parts: Vec<&str> = group.splitn(2, ' ').collect();
                let kind = parts[0];
                let content = parts[1];

                match kind {
                    "json" => Ok(Some(ResponseHandler::Json { json_path: content.into() })),
                    unknown => Err(FhttpError::new(format!("Unknown response handler '{}'", unknown)))
                }
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{Result, Request};
    use indoc::indoc;

    #[test]
    fn response_handler() -> Result<()> {
        let req = Request::new(std::env::current_dir().unwrap(), indoc!(r##"
            POST http://localhost:8080

            this is the body

            > {%
                json $
            %}
        "##));

        assert!(req.response_handler()?.is_some());

        Ok(())
    }

}
