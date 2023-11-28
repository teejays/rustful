use std::{
    any::Any,
    boxed::Box,
    collections::HashMap,
    fmt::{self, format},
    io::Error,
    io::{BufRead, BufReader, ErrorKind, Write},
    net::{TcpListener, TcpStream},
    sync::OnceLock,
};

use regex::Regex;

use crate::rest_server;

pub enum HttpMethod {
    GET,
    POST,
}

impl fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            rest_server::HttpMethod::GET => "GET".to_string(),
            rest_server::HttpMethod::POST => "POST".to_string(),
        };
        write!(f, "[{s}]")
    }
}

pub struct HttpRequest<'a> {
    method: HttpMethod,
    path: &'a str,
    headers: HashMap<String, String>,
    body: String,
}

type HandlerFunc = fn(req: HttpRequest) -> Result<Box<dyn Any>, Error>;

/// RestServer implements a Restful HTTP server.
pub struct RestServer<'a> {
    name: &'a str,
    addr: &'a str,
    port: u16,
    path_handler_map: HashMap<&'a str, HandlerFunc>,
}

const HTTP_REGEX_PATTERN: &str = r"(GET|POST|OPTION|PUT|DELETE)\s(\/[\S]*)\s([\S]+)$";

fn http_regex() -> &'static Regex {
    static HTTP_REQ_REGEX: OnceLock<Regex> = OnceLock::new();
    return HTTP_REQ_REGEX.get_or_init(|| Regex::new(HTTP_REGEX_PATTERN).unwrap());
}

impl<'a> RestServer<'a> {
    /// Create a new RestServer
    pub fn new(name: &'a str, addr: &'a str, port: u16) -> Result<Self, Error> {
        if name == "" {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "RestServer: cannot create a new server with empty name",
            ));
        }
        return Ok(RestServer {
            name,
            addr,
            port,
            path_handler_map: HashMap::new(),
        });
    }

    /// Adds a handler to the specified path
    pub fn register_path(&mut self, path: &'a str, func: HandlerFunc) -> Result<(), Error> {
        if self.path_handler_map.contains_key(path) {
            return Err(Error::new(
                ErrorKind::Other,
                format!(
                    "HttpServer [{0}] path [{path}]: attempted to set handler twice",
                    self.name
                ),
            ));
        }
        let _ = self.path_handler_map.insert(path, func);
        return Ok(());
    }

    pub fn listen(&self) -> Result<(), Error> {
        let port_str = self.port.to_string();
        let full_addr = self.addr.to_owned() + ":" + &port_str;

        // Start the listener
        let listener = TcpListener::bind(full_addr)?;

        // Listen for packets
        for stream_result in listener.incoming() {
            // If detect packet, read the entire request
            match stream_result {
                Ok(stream) => match self.handle_connection(stream) {
                    Ok(()) => continue,
                    Err(err) => println!("Error in handling connection: {err}"),
                },
                Err(err) => {
                    println!("Error in connection: {err}");
                }
            };
        }

        let _ = listener.accept()?;
        return Ok(());
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<(), Error> {
        println!("Connection established!\nRequest: {:?}", stream);

        // handle_request(self, &mut stream);

        let buf_reader = BufReader::new(&stream);
        let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| {
                let str = result.unwrap();
                println!("- Request line: {str}");
                return str;
            })
            .take_while(|line| !line.is_empty())
            .collect();

        println!("Request received:\n{:?}", http_request);

        // Parse the request to get method, path etc.
        if http_request.len() < 1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "HTTP request is invalid",
            ));
        }

        let re = http_regex();
        let http_captures: regex::Captures<'_>;
        match re.captures(&http_request[0]) {
            Some(cs) => http_captures = cs,
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!(
                        "HTTP request is invalid - no parts found in line: {0}",
                        http_request[0]
                    ),
                ))
            }
        }
        println!("Parsed caputures: {:?}", http_captures);
        if http_captures.len() != 4 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                    "HTTP request is invalid: expected 4 parts but found {:?}: {:#?}",
                    http_captures.len(),
                    http_captures,
                ),
            ));
        }
        let method = match &http_captures[1] {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            s => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("HTTP request is invalid: unexpected method: {s}"),
                ))
            }
        };
        let path = &http_captures[2];
        let protocol = &http_captures[3];

        println!("Method: {method}");
        println!("Path: {path}");
        println!("Protocol: {protocol}");

        if !protocol.contains("HTTP") {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!(
                "HTTP request is invalid: expected protocol to be HTTP but got {protocol}: {:#?}",
                http_captures
            ),
            ));
        }

        let http_request: HttpRequest = HttpRequest {
            method: method,
            path: path,
            headers: HashMap::new(),
            body: "todo".to_string(),
        };

        // Find the handler for this path
        // Todo: split the path into the path + vars + params etc.

        let resp = match self.path_handler_map.get(path) {
            Some(handler) => handler(http_request),
            None => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("HTTP request is invalid: No handler found for path {path}",),
                ))
            }
        };

        let resp_str = match resp {
            Ok(r) => format!("{:#?}", r),
            Err(err) => format!("error: {err}"),
        };

        println!("Response String: {:?}", resp_str);

        // Write to the stream and close
        let response = format!("HTTP/1.1 200 OK\n{resp_str}\r\n\r\n");

        stream.write_all(response.as_bytes())?;

        return Ok(());
    }
}

// Handler for /ping
pub fn handle_ping(req: HttpRequest) -> Result<Box<dyn Any>, Error> {
    println!("Handling ping");
    return Ok(Box::new("pong"));
}
