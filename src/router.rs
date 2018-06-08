use super::{Handler, Matcher, Route};

#[allow(dead_code)]
pub struct Router {
    options_routes: Vec<Route>,
    get_routes: Vec<Route>,
    post_routes: Vec<Route>,
    put_routes: Vec<Route>,
    delete_routes: Vec<Route>,
    head_routes: Vec<Route>,
    trace_routes: Vec<Route>,
    connect_routes: Vec<Route>,
    patch_routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Router {
        Router {
            options_routes: vec![],
            get_routes: vec![],
            post_routes: vec![],
            put_routes: vec![],
            delete_routes: vec![],
            head_routes: vec![],
            trace_routes: vec![],
            connect_routes: vec![],
            patch_routes: vec![],
        }
    }

    pub fn options(mut self, matcher: Box<Matcher>, handler: Handler) -> Router {
        // path: &str
        // let mut regex = "^".to_string();
        // regex.push_str(path);
        // regex.push_str("$");
        // Path { matcher: Regex::new(&regex).unwrap() }
        self.options_routes.push(Route::new(matcher, handler));
        self
    }
}
