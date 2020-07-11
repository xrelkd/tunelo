use std::time::Duration;

use url::Url;

use tunelo::{
    checker::{BasicProber, HttpProber, SimpleProxyChecker},
    common::{HostAddress, ProxyHost},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut task = SimpleProxyChecker::new(ProxyHost::Socks5 {
        host: "127.96.0.3".to_owned(),
        port: 3128,
        username: None,
        password: None,
    });

    task.add_prober(BasicProber::new(HostAddress::new("www.google.com", 80)).into());
    task.add_prober(BasicProber::new(HostAddress::new("www.google.com", 443)).into());
    task.add_prober(HttpProber::get(Url::parse("https://www.google.com/").unwrap(), 200).into());
    task.add_prober(HttpProber::head(Url::parse("https://ifconfig.me/ip").unwrap(), 200).into());
    task.add_prober(HttpProber::head(Url::parse("http://httpbin.org/get").unwrap(), 200).into());
    task.add_prober(
        HttpProber::delete(Url::parse("http://httpbin.org/delete").unwrap(), 200).into(),
    );

    let report = task.run_parallel(Some(Duration::from_secs(3))).await;
    println!("{:?}", report);
    Ok(())
}

#[cfg(test)]
mod tests {
    fn it_works() {
        let buf = r#"HTTP/1.0 200 OK
Date: Mon, 06 Jul 2020 12:24:26 GMT
Content-Type: text/plain; charset=utf-8
Content-Length: 12
Access-Control-Allow-Origin: *
Via: 1.1 google

159.89.49.60"#;
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut response = httparse::Response::new(&mut headers);

        let res = response.parse(buf.as_bytes()).unwrap();
        // assert_eq!(res, 0);
        if res.is_complete() {
            // assert_eq!(response.version, Some("HTTP/1.0".as_bytes()));
            assert_eq!(response.code, Some(200));
            // match res.version {
            //     Some(ref path) => {
            //         assert_eq!();
            //         // check router for path.
            //         // /404 doesn't exist? we could stop parsing
            //     }
            //     None => {
            //         // must read more and parse again
            //     }
            // }
        }
    }
}
