use futures::BoxFuture;
use sparkles::{Request, Response, ResponseBuilder, Status, Error};

/// The handler for /
pub fn root(_: Request) -> BoxFuture<Response, Error> {
    let mut res = ResponseBuilder::new();
    res.with_template("index".to_string());


/*
    res.data.insert("releases".to_string(),
                Value::Array(thanks::releases::all()));
                */

    res.with_status(Status::Ok);

    res.to_response().into_future()
}