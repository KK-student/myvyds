use serde_json::Value;

///A trait to convert JSON data into an object that implements the trait.
pub trait FromJson {
    //Convert the given JSON data (json) to an object of whatever the `Self` type is.
    ///
    /// Parameters:
    ///     json: The given JSON data.
    fn from_json(json: &Value) -> Self;
}
