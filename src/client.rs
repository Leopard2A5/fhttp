use reqwest::blocking::Client as InnerClient;
use reqwest::Url;
use crate::Request;

pub struct Client;

impl Client {
    pub fn new() -> Self {
        Client {}
    }

    pub fn exec(
        &self,
        request: Request
    ) {
        let client: InnerClient = InnerClient::new();
        let url = Url::parse(&request.url).unwrap();
        let req = client
            .request(request.method, url)
            .headers(request.headers)
            .body(request.body);
        let response = req.send().unwrap();
        println!("{:?}", response.status());
        println!("{:?}", response.text().unwrap());
    }
}
