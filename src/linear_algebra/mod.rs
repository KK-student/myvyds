use libm::sqrt;

#[derive(Debug)]
pub struct MathError {
    message: String,
}

impl MathError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

fn dot_product_cpu(a: &[f64], b: &[f64]) -> Result<f64, MathError> {
    if (a.len() != b.len()) || a.is_empty() || b.is_empty() {
        return Err(MathError {
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

pub fn dot_product(a: &[f64], b: &[f64]) -> Result<f64, MathError> {
    dot_product_cpu(a, b)
}

pub fn magnitude(a: &[f64]) -> Result<f64, MathError> {
    let dp: f64 = match dot_product(a, a) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    Ok(sqrt(dp))
}

pub fn cosine_similarity(a: &[f64], b: &[f64]) -> Result<f64, MathError> {
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
