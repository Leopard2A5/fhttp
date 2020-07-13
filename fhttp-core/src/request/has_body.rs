use std::borrow::Cow;

use crate::{Request, Result, FhttpError};
use crate::request::body::Body;
use serde_json::Value;
use serde_json::map::Map;

pub trait HasBody {
    fn body(&self) -> Result<Body>;
}

impl Request {
    fn _body(&self) -> Result<&str> {
        let mut body_start = None;
        let mut body_end = None;
        let mut text_index: usize = 0;
        let mut last_char = None;

        for (index, chr) in self.text.chars().enumerate() {
            if body_start.is_none() && chr == '\n' && last_char == Some('\n') {
                body_start = Some(text_index + 1);
            } else if body_end.is_none() && chr == '%' && &self.text[(index - 4)..index] == "\n> {" {
                body_end = Some(index - 4);
                break;
            }

            last_char = Some(chr);
            text_index += 1;
        }

        match body_start {
            Some(start) => {
                let end = body_end.unwrap_or(text_index);
                if start < end {
                    Ok(&self.text[start..body_end.unwrap_or(text_index)])
                } else {
                    Ok("")
                }
            },
            None => Ok(""),
        }
    }

    fn _gql_body(&self) -> Result<String> {
        let body = self._body()?;
        let parts: Vec<&str> = body.split("\n\n").collect();

        let (query, variables) = match parts.len() {
            1 => (parts[0], None),
            2 => (parts[0], Some(parse_variables(parts[1])?)),
            _ => return Err(FhttpError::new("GraphQL requests can only have 1 or 2 body parts")),
        };

        let query = Value::String(query.to_owned());

        let mut map = Map::new();
        map.insert("query".into(), query);
        map.insert("variables".into(), variables.unwrap_or(Value::Object(Map::new())));
        let body = Value::Object(map);

        Ok(serde_json::to_string(&body).unwrap())
    }
}

impl HasBody for Request {
    fn body(&self) -> Result<Body> {
        Ok(Body::Plain(
            match self.gql_file() {
                true => Cow::Owned(self._gql_body()?),
                false => Cow::Borrowed(self._body()?),
            }
        ))
    }
}

fn parse_variables(text: &str) -> Result<Value> {
    serde_json::from_str::<Value>(&text)
        .map_err(|_| FhttpError::new("Error parsing variables"))
}
