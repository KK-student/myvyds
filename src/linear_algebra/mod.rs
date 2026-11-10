use libm::sqrt;

#[derive(Debug)]
pub struct GenericError {
    message: String,
}

impl GenericError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

///Uses the cpu to get the dot product of 2 vectors. Returns a GenericError if the vectors are not of the same length or the length is 0.
fn dot_product_cpu(a: &[f64], b: &[f64]) -> Result<f64, GenericError> {
    if (a.len() != b.len()) || a.is_empty() || b.is_empty() {
        return Err(GenericError {
            message: "Invalid vector sizes!".to_string(),
        });
    }
    let length: usize = a.len();
    let mut sum: f64 = 0.0;
    let mut i: usize = 0;
    while i < length {
        sum += a[i] * b[i];
        i += 1;
    }

    Ok(sum)
}

///Get the dot product of 2 vectors. Returns a GenericError if the vectors are not of the same length or the length is 0.
pub fn dot_product(a: &[f64], b: &[f64]) -> Result<f64, GenericError> {
    dot_product_cpu(a, b)
}

///Gets the magnitude of a vector.
pub fn magnitude(a: &[f64]) -> Result<f64, GenericError> {
    let dp: f64 = match dot_product(a, a) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    Ok(sqrt(dp))
}

///Gets the cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> Result<f64, GenericError> {
    let dp_ab: f64 = match dot_product(a, b) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mag_a: f64 = match magnitude(a) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    let mag_b: f64 = match magnitude(b) {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    Ok(dp_ab / (mag_a * mag_b))
}
