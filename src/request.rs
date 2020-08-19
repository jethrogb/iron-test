/// Contains convenience methods for making requests to Iron Handlers.
use hyper;
use hyper::buffer::BufReader;
use hyper::net::NetworkStream;

use iron;
use iron::prelude::*;
use iron::{Handler, headers, Headers, method, Url};

use std::io::Cursor;

use super::mock_stream::MockStream;
use std::net::SocketAddr;

/// Convenience method for making GET requests to Iron Handlers.
pub fn get<H: Handler>(path: &str, headers: Headers, handler: &H) -> IronResult<Response> {
    request(None, method::Get, path, "", headers, handler)
}

/// Convenience method for making POST requests with a body to Iron Handlers.
pub fn post<H: Handler>(path: &str, headers: Headers, body: &str, handler: &H) -> IronResult<Response> {
    request(None, method::Post, path, body, headers, handler)
}

/// Convenience method for making PATCH requests with a body to Iron Handlers.
pub fn patch<H: Handler>(path: &str, headers: Headers, body: &str, handler: &H) -> IronResult<Response> {
    request(None, method::Patch, path, body, headers, handler)
}

/// Convenience method for making PUT requests with a body to Iron Handlers.
pub fn put<H: Handler>(path: &str, headers: Headers, body: &str, handler: &H) -> IronResult<Response> {
    request(None, method::Put, path, body, headers, handler)
}

/// Convenience method for making DELETE requests to Iron Handlers.
pub fn delete<H: Handler>(path: &str, headers: Headers, handler: &H) -> IronResult<Response> {
    request(None, method::Delete, path, "", headers, handler)
}

/// Convenience method for making OPTIONS requests to Iron Handlers.
pub fn options<H: Handler>(path: &str, headers: Headers, handler: &H) -> IronResult<Response> {
    request(None, method::Options, path, "", headers, handler)
}

/// Convenience method for making HEAD requests to Iron Handlers.
pub fn head<H: Handler>(path: &str, headers: Headers, handler: &H) -> IronResult<Response> {
    request(None, method::Head, path, "", headers, handler)
}

/// Constructs an Iron::Request from the given parts and passes it to the
/// `handle` method on the given Handler.
pub fn request<H: Handler>(client_addr: Option<SocketAddr>,
                           method: method::Method,
                           path: &str,
                           body: &str,
                           headers: Headers,
                           handler: &H) -> IronResult<Response> {
    let url = Url::parse(path).unwrap();
    // From iron 0.5.x, iron::Request contains private field. So, it is not good to
    // create iron::Request directly. Make http request and parse it with hyper,
    // and make iron::Request from hyper::client::Request.
    let mut buffer = String::new();
    buffer.push_str(&format!("{} {} HTTP/1.1\r\n", &method, url));
    buffer.push_str(&format!("Content-Length: {}\r\n", body.len() as u64));
    for header in headers.iter() {
        buffer.push_str(&format!("{}: {}\r\n", header.name(), header.value_string()));
    }
    if !headers.has::<headers::UserAgent>() {
        buffer.push_str(&format!("User-Agent: iron-test\r\n"));
    }
    buffer.push_str("\r\n");
    buffer.push_str(body);

    let protocol = match url.scheme() {
        "http" => iron::Protocol::http(),
        "https" => iron::Protocol::https(),
        _ => panic!("unknown protocol {}", url.scheme()),
    };

    let url = url::Url::parse(path).unwrap();

    // Lookup the host string, if it's a domain just convert it to 127.0.0.1, if it's an ip address keep it.
    let server_ip : std::net::IpAddr = url.host_str().unwrap().parse().or_else(|_| "127.0.0.1".parse()).unwrap();
    let server_addr = SocketAddr::new(server_ip, url.port_or_known_default().unwrap_or(80));

    let client_addr = client_addr.unwrap_or("127.0.0.1:3000".parse().unwrap());
    let mut stream = MockStream::new(client_addr, Cursor::new(buffer.as_bytes().to_vec()));
    let mut buf_reader = BufReader::new(&mut stream as &mut dyn NetworkStream);


    let http_request = hyper::server::Request::new(&mut buf_reader, client_addr).unwrap();
    let mut req = Request::from_http(http_request, server_addr, &protocol).unwrap();

    handler.handle(&mut req)
}

#[cfg(test)]
mod test {
    extern crate router;
    extern crate urlencoded;

    use iron::headers::Headers;
    use iron::mime::Mime;
    use iron::prelude::*;
    use iron::{Handler, headers, status};

    use response::{extract_body_to_bytes, extract_body_to_string};

    use self::urlencoded::UrlEncodedBody;

    use super::*;

    struct HelloWorldHandler;

    impl Handler for HelloWorldHandler {
        fn handle(&self, _: &mut Request) -> IronResult<Response> {
            Ok(Response::with((status::Ok, "Hello, world!")))
        }
    }

    struct RouterHandler;

    impl Handler for RouterHandler {
        fn handle(&self, req: &mut Request) -> IronResult<Response> {
            let params = req.extensions
                .get::<router::Router>()
                .expect("Expected to get a Router from the request extensions.");
            let id = params.find("id").unwrap();

            Ok(Response::with((status::Ok, id)))
        }
    }

    struct PostHandler;

    impl Handler for PostHandler {
        fn handle(&self, req: &mut Request) -> IronResult<Response> {
            let body = req.get_ref::<UrlEncodedBody>()
                .expect("Expected to extract a UrlEncodedBody from the request");
            let first_name = body.get("first_name").unwrap()[0].to_owned();
            let last_name = body.get("last_name").unwrap()[0].to_owned();

            Ok(Response::with((status::Ok, first_name + " " + &last_name)))
        }
    }

    struct UpdateHandler;

    impl Handler for UpdateHandler {
        fn handle(&self, req: &mut Request) -> IronResult<Response> {
            let id = {
                let params = req.extensions
                    .get::<router::Router>()
                    .expect("Expected to get a Router from request extensions.");
                params.find("id").unwrap().parse::<String>().unwrap()
            };

            let body = req.get_ref::<UrlEncodedBody>()
                .expect("Expected to extract a UrlEncodedBody from the request");
            let first_name = body.get("first_name").unwrap()[0].to_owned();
            let last_name = body.get("last_name").unwrap()[0].to_owned();

            Ok(Response::with((status::Ok, [first_name, last_name, id].join(" "))))
        }
    }

    struct OptionsHandler;

    impl Handler for OptionsHandler {
        fn handle(&self, _: &mut Request) -> IronResult<Response> {
            Ok(Response::with((status::Ok, "ALLOW: GET,POST")))
        }
    }

    struct HeadHandler;

    impl Handler for HeadHandler {
        fn handle(&self, _: &mut Request) -> IronResult<Response> {
            Ok(Response::with(status::Ok))
        }
    }

    struct UserAgentHandler;

    impl Handler for UserAgentHandler {
        fn handle(&self, req: &mut Request) -> IronResult<Response> {
            let user_agent = req.headers.get::<headers::UserAgent>().unwrap();
            Ok(Response::with((status::Ok, user_agent.to_string())))
        }
    }

    struct ClientAddrHandler;

    impl Handler for ClientAddrHandler {
        fn handle(&self, req: &mut Request) -> IronResult<Response> {
            Ok(Response::with((status::Ok, req.remote_addr.to_string())))
        }
    }

    struct ServerAddrHandler;

    impl Handler for ServerAddrHandler {
        fn handle(&self, req: &mut Request) -> IronResult<Response> {
            Ok(Response::with((status::Ok, req.local_addr.to_string())))
        }
    }

    #[test]
    fn test_get() {
        let response = get("http://127.0.0.1:3000", Headers::new(), &HelloWorldHandler);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_post() {
        let mut headers = Headers::new();
        let mime: Mime = "application/x-www-form-urlencoded".parse().unwrap();
        headers.set(headers::ContentType(mime));
        let response = post("http://127.0.0.1:3000/users",
                            headers,
                            "first_name=Example&last_name=User",
                            &PostHandler);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"Example User");
    }

    #[test]
    fn test_patch() {
        let mut router = router::Router::new();
        router.patch("/users/:id", UpdateHandler, "update");

        let mut headers = Headers::new();
        let mime: Mime = "application/x-www-form-urlencoded".parse().unwrap();
        headers.set(headers::ContentType(mime));
        let response = patch("http://127.0.0.1:3000/users/1",
                             headers,
                             "first_name=Example&last_name=User",
                             &router);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"Example User 1");
    }

    #[test]
    fn test_put() {
        let mut router = router::Router::new();
        router.put("/users/:id", UpdateHandler, "update");

        let mut headers = Headers::new();
        let mime: Mime = "application/x-www-form-urlencoded".parse().unwrap();
        headers.set(headers::ContentType(mime));
        let response = put("http://127.0.0.1:3000/users/2",
                           headers,
                           "first_name=Example&last_name=User",
                           &router);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"Example User 2");
    }

    #[test]
    fn test_delete() {
        let mut router = router::Router::new();
        router.delete("/:id", RouterHandler, "update");

        let response = delete("http://127.0.0.1:3000/1", Headers::new(), &router);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"1");
    }


    #[test]
    fn test_options() {
        let response = options("http://127.0.0.1:3000/users/options", Headers::new(), &OptionsHandler);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"ALLOW: GET,POST");
    }

    #[test]
    fn test_head() {
        let response = head("http://127.0.0.1:3000/users", Headers::new(), &HeadHandler);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"");
    }
    
    #[test]
    fn test_head_domain() {
        let response = head("http://test/users", Headers::new(), &HeadHandler);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"");
    }

    #[test]
    fn test_user_agent_not_provided() {
        let headers = Headers::new();
        let response = get("http://127.0.0.1:3000/", headers, &UserAgentHandler);
        let result = extract_body_to_string(response.unwrap());

        assert_eq!(result, "iron-test");
    }

    #[test]
    fn test_user_agent_provided() {
        let mut headers = Headers::new();
        headers.set(headers::UserAgent("CustomAgent/1.0".to_owned()));
        let response = get("http://127.0.0.1:3000/", headers, &UserAgentHandler);
        let result = extract_body_to_string(response.unwrap());

        assert_eq!(result, "CustomAgent/1.0");
    }

    #[test]
    fn test_percent_decoded_url() {
        let response = head("http://127.0.0.1:3000/some path with spaces", Headers::new(), &HeadHandler);
        let result = extract_body_to_bytes(response.unwrap());

        assert_eq!(result, b"");
    }

    #[test]
    fn test_client_addr() {
        let response = request(Some("127.0.0.1:1234".parse().unwrap()), method::Method::Get, "http://127.0.0.1:3000/", "", Headers::new(), &ClientAddrHandler);
        let result = extract_body_to_string(response.unwrap());

        assert_eq!(result, "127.0.0.1:1234");
    }

    #[test]
    fn test_client_addr_default() {
        let response = request(None, method::Method::Get, "http://127.0.0.1:3000/", "", Headers::new(), &ClientAddrHandler);
        let result = extract_body_to_string(response.unwrap());

        assert_eq!(result, "192.0.2.1:3000");
    }

    #[test]
    fn test_server_addr() {
        let response = request(None, method::Method::Get, "http://127.0.0.1:80/", "", Headers::new(), &ServerAddrHandler);
        let result = extract_body_to_string(response.unwrap());

        assert_eq!(result, "127.0.0.1:80");
    }
}
