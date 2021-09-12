//! HTTP server components for poet.

#[rocket::get("/")]
fn index() -> &'static str {
    "Hello from poet!"
}

pub async fn run() {
    println!("*****************************************************************");
    println!("*                                                               *");
    println!("*  Launching Web Server.                                        *");
    println!("*                                                               *");
    println!("*  Type Control-C in the Terminal to stop the server.           *");
    println!("*                                                               *");
    println!("*****************************************************************");

    let result = rocket::build().mount("/", routes![index]).launch().await;
    if let Err(e) = result {
        println!("***** Failed to launch web server. *****");
        // Drop the error to get a Rocket-formatted panic.
        drop(e);
    };
}
