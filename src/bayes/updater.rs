use std::collections::HashMap;

pub fn bayes_update(priors: &mut HashMap<String, f32>, likelihoods: &HashMap<String, f32>) {
    // unnormalized posterior
    for (hypothesis, prior) in priors.iter_mut() {
        // If a likelihood isn't explicitly provided, assume it's low (decay).
        // This ensures the hypotheses that DO match the text will dominate.
        let likelihood = likelihoods.get(hypothesis).copied().unwrap_or(0.1);
        *prior *= likelihood;
    }

    // normalize
    let total: f32 = priors.values().sum();
    if total <= f32::EPSILON {
        return;
    }
    for value in priors.values_mut() {
        *value /= total;
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

        bayes_update(&mut priors, &likelihoods);

        // A prior: 0.5 * 0.8 = 0.4
        // B prior: 0.5 * 0.2 = 0.1
        // Total: 0.5
        // Normalized A: 0.4 / 0.5 = 0.8
        // Normalized B: 0.1 / 0.5 = 0.2
        assert!((priors["A"] - 0.8).abs() < f32::EPSILON);
        assert!((priors["B"] - 0.2).abs() < f32::EPSILON);
    }
}
