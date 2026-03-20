/// SIMD-optimized dot product operations.
///
/// Uses NEON intrinsics on aarch64 for 4-wide f32 and 2-wide f64 operations.
/// Falls back to scalar code on other architectures.

/// Compute dot product of two f32 slices, using NEON on aarch64.
#[inline]
pub fn dot_f32(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "dot_f32: slice lengths must match");
    dot_f32_impl(a, b)
}

/// Compute dot product of two f64 slices, using NEON on aarch64.
#[inline]
pub fn dot_f64(a: &[f64], b: &[f64]) -> f64 {
    assert_eq!(a.len(), b.len(), "dot_f64: slice lengths must match");
    dot_f64_impl(a, b)
}

// ── aarch64 NEON implementations ──

#[cfg(target_arch = "aarch64")]
#[inline]
fn dot_f32_impl(a: &[f32], b: &[f32]) -> f32 {
    use std::arch::aarch64::*;

    let len = a.len();
    let chunks = len / 4;
    let remainder = len % 4;

    let mut acc = unsafe {
        let mut acc = vdupq_n_f32(0.0);
        for i in 0..chunks {
            let offset = i * 4;
            let va = vld1q_f32(a.as_ptr().add(offset));
            let vb = vld1q_f32(b.as_ptr().add(offset));
            acc = vfmaq_f32(acc, va, vb);
        }
        vaddvq_f32(acc)
    };

    // Handle remainder elements
    let tail_start = chunks * 4;
    for i in 0..remainder {
        acc += a[tail_start + i] * b[tail_start + i];
    }

    acc
}

#[cfg(target_arch = "aarch64")]
#[inline]
fn dot_f64_impl(a: &[f64], b: &[f64]) -> f64 {
    use std::arch::aarch64::*;

    let len = a.len();
    let chunks = len / 2;
    let remainder = len % 2;

    let mut acc = unsafe {
        let mut acc = vdupq_n_f64(0.0);
        for i in 0..chunks {
            let offset = i * 2;
            let va = vld1q_f64(a.as_ptr().add(offset));
            let vb = vld1q_f64(b.as_ptr().add(offset));
            acc = vfmaq_f64(acc, va, vb);
        }
        vaddvq_f64(acc)
    };

    // Handle remainder element
    if remainder == 1 {
        acc += a[len - 1] * b[len - 1];
    }

    acc
}

// ── Scalar fallback for non-aarch64 ──

#[cfg(not(target_arch = "aarch64"))]
#[inline]
fn dot_f32_impl(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0f32;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

#[cfg(not(target_arch = "aarch64"))]
#[inline]
fn dot_f64_impl(a: &[f64], b: &[f64]) -> f64 {
    let mut sum = 0.0f64;
    for i in 0..a.len() {
        sum += a[i] * b[i];
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_f32_basic() {
        let a = [1.0f32, 2.0, 3.0, 4.0];
        let b = [5.0f32, 6.0, 7.0, 8.0];
        let result = dot_f32(&a, &b);
        // 1*5 + 2*6 + 3*7 + 4*8 = 5 + 12 + 21 + 32 = 70
        assert!((result - 70.0).abs() < 1e-6, "Expected 70.0, got {}", result);
    }

    #[test]
    fn test_dot_f32_with_remainder() {
        let a = [1.0f32, 2.0, 3.0, 4.0, 5.0];
        let b = [2.0f32, 3.0, 4.0, 5.0, 6.0];
        let result = dot_f32(&a, &b);
        // 1*2 + 2*3 + 3*4 + 4*5 + 5*6 = 2 + 6 + 12 + 20 + 30 = 70
        assert!((result - 70.0).abs() < 1e-6, "Expected 70.0, got {}", result);
    }

    #[test]
    fn test_dot_f32_empty() {
        let a: [f32; 0] = [];
        let b: [f32; 0] = [];
        assert_eq!(dot_f32(&a, &b), 0.0);
    }

    #[test]
    fn test_dot_f64_basic() {
        let a = [1.0f64, 2.0, 3.0, 4.0];
        let b = [5.0f64, 6.0, 7.0, 8.0];
        let result = dot_f64(&a, &b);
        assert!((result - 70.0).abs() < 1e-12, "Expected 70.0, got {}", result);
    }

    #[test]
    fn test_dot_f64_with_remainder() {
        let a = [1.0f64, 2.0, 3.0];
        let b = [4.0f64, 5.0, 6.0];
        let result = dot_f64(&a, &b);
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!((result - 32.0).abs() < 1e-12, "Expected 32.0, got {}", result);
    }

    #[test]
    fn test_dot_f64_empty() {
        let a: [f64; 0] = [];
        let b: [f64; 0] = [];
        assert_eq!(dot_f64(&a, &b), 0.0);
    }

    #[test]
    fn test_dot_f32_large() {
        // Test with a size that exercises multiple NEON chunks plus remainder
        let n = 613; // MLP_INPUT_SIZE
        let a: Vec<f32> = (0..n).map(|i| (i as f32) * 0.01).collect();
        let b: Vec<f32> = (0..n).map(|i| ((n - i) as f32) * 0.01).collect();
        let simd_result = dot_f32(&a, &b);
        let scalar_result: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        assert!(
            (simd_result - scalar_result).abs() < 1e-2,
            "Large f32 dot product mismatch: simd={}, scalar={}",
            simd_result, scalar_result
        );
    }

    #[test]
    fn test_dot_f64_large() {
        let n = 613;
        let a: Vec<f64> = (0..n).map(|i| (i as f64) * 0.01).collect();
        let b: Vec<f64> = (0..n).map(|i| ((n - i) as f64) * 0.01).collect();
        let simd_result = dot_f64(&a, &b);
        let scalar_result: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        assert!(
            (simd_result - scalar_result).abs() < 1e-8,
            "Large f64 dot product mismatch: simd={}, scalar={}",
            simd_result, scalar_result
        );
    }
}
