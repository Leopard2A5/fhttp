use std::fmt::Debug;

pub trait ResponseHandler : Debug {
    fn process_body(
        &self,
        body: &str
    ) -> String;
}

#[derive(Debug)]
pub struct JsonPathResponseHandler {
    json_path: String,
}

impl JsonPathResponseHandler {
    pub fn new(json_path: &str) -> Self {
        JsonPathResponseHandler { json_path: json_path.to_owned() }
    }
}

impl ResponseHandler for JsonPathResponseHandler {
    fn process_body(
        &self,
        body: &str
    ) -> String {
        unimplemented!()
    }
}

//data class JsonPathResponseHandler(
//    private val jsonPath: String
//) : ResponseHandler {
//    override fun processBody(body: String): String {
//        return JsonPath.read<Any>(body, jsonPath).toString()
//    }
//}
