use std::collections::HashMap;

/// Optimized Bayesian update (Task 268.2).
/// 
/// - Early exit when there are no likelihoods (common for generic chat).
/// - Single combined pass where possible.
/// - Avoids repeated HashMap lookups.
pub fn bayes_update(
    priors: &mut HashMap<String, f32>,
    likelihoods: &HashMap<String, f32>,
    decay_rate: f32,
    pull_rate: f32,
) {
    crate::perf_guard!("bayes.bayes_update");

    // Task 268.2 optimization: fast path for no new evidence
    if likelihoods.is_empty() {
        // Still apply gentle decay toward long-term priors
        if decay_rate < 1.0 || pull_rate > 0.0 {
            let mut total = 0.0f32;
            for (_intent, belief) in priors.iter_mut() {
                let long_term = 0.0; // we don't have the original prior here, so just decay
                *belief = *belief * decay_rate + long_term * pull_rate;
                if *belief < 0.001 { *belief = 0.001; }
                total += *belief;
            }
            if total > f32::EPSILON {
                for v in priors.values_mut() {
                    *v /= total;
                }
            }
        }
        return;
    }

    // Combined update + clamp pass
    let mut total = 0.0f32;
    for (hypothesis, prior) in priors.iter_mut() {
        let likelihood = likelihoods.get(hypothesis).copied().unwrap_or(0.75);
        let mut val = *prior * likelihood;

        // Decay / pull toward "prior evidence" (likelihoods can act as soft prior here)
        let evidence = likelihoods.get(hypothesis).copied().unwrap_or(0.0);
        val = val * decay_rate + evidence * pull_rate;

        if val < 0.001 {
            val = 0.001;
        }
        *prior = val;
        total += val;
    }

    // Normalize in one pass
    if total > f32::EPSILON {
        for value in priors.values_mut() {
            *value /= total;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bayes_update() {
        let mut priors = HashMap::new();
        priors.insert("A".to_string(), 0.5);
        priors.insert("B".to_string(), 0.5);

        let mut likelihoods = HashMap::new();
        likelihoods.insert("A".to_string(), 0.8); // Strong evidence for A
        likelihoods.insert("B".to_string(), 0.2); // Weak evidence for B

        bayes_update(&mut priors, &likelihoods, 0.95, 0.05);

        // A prior: 0.5 * 0.8 = 0.4
        // B prior: 0.5 * 0.2 = 0.1
        // Total: 0.5
        // Normalized A: 0.4 / 0.5 = 0.8
        // Normalized B: 0.1 / 0.5 = 0.2
        assert!((priors["A"] - 0.8).abs() < f32::EPSILON);
        assert!((priors["B"] - 0.2).abs() < f32::EPSILON);
    }
}
