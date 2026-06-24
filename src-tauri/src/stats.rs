//! Inter-rater reliability statistics.
//!
//! Cohen's kappa measures agreement between two raters on categorical
//! decisions, correcting for chance. Used to report how consistently the two
//! AI coding raters assign codes across the corpus.

/// Cohen's kappa and the raw observed-agreement proportion for paired
/// categorical decisions. Returns `None` for an empty input. When neither rater
/// varies, chance agreement is total — we return kappa = 1.0 if they fully agree
/// and 0.0 otherwise (the conventional reading).
pub fn cohens_kappa(pairs: &[(String, String)]) -> Option<(f64, f64)> {
    let n = pairs.len();
    if n == 0 {
        return None;
    }
    let n_f = n as f64;
    let agree = pairs.iter().filter(|(a, b)| a == b).count() as f64;
    let p_o = agree / n_f;

    let mut cats: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for (a, b) in pairs {
        cats.insert(a.as_str());
        cats.insert(b.as_str());
    }
    let mut p_e = 0.0;
    for c in &cats {
        let a_c = pairs.iter().filter(|(a, _)| a == c).count() as f64 / n_f;
        let b_c = pairs.iter().filter(|(_, b)| b == c).count() as f64 / n_f;
        p_e += a_c * b_c;
    }

    let kappa = if (1.0 - p_e).abs() < f64::EPSILON {
        if p_o >= 1.0 - f64::EPSILON { 1.0 } else { 0.0 }
    } else {
        (p_o - p_e) / (1.0 - p_e)
    };
    Some((kappa, p_o))
}

/// Landis & Koch (1977) qualitative band for a kappa value.
pub fn kappa_interpretation(kappa: f64) -> &'static str {
    if kappa < 0.0 {
        "Poor (worse than chance)"
    } else if kappa <= 0.20 {
        "Slight"
    } else if kappa <= 0.40 {
        "Fair"
    } else if kappa <= 0.60 {
        "Moderate"
    } else if kappa <= 0.80 {
        "Substantial"
    } else {
        "Almost perfect"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn empty_is_none() {
        assert!(cohens_kappa(&[]).is_none());
    }

    #[test]
    fn perfect_agreement_with_variance() {
        let p = pairs(&[("1", "1"), ("0", "0"), ("1", "1"), ("0", "0")]);
        let (k, po) = cohens_kappa(&p).unwrap();
        assert!((po - 1.0).abs() < 1e-9);
        assert!((k - 1.0).abs() < 1e-9);
    }

    #[test]
    fn no_variance_full_agreement_is_one() {
        let p = pairs(&[("1", "1"), ("1", "1")]);
        let (k, _) = cohens_kappa(&p).unwrap();
        assert!((k - 1.0).abs() < 1e-9);
    }

    #[test]
    fn total_disagreement_is_negative() {
        let p = pairs(&[("1", "0"), ("0", "1"), ("1", "0"), ("0", "1")]);
        let (k, po) = cohens_kappa(&p).unwrap();
        assert!(po.abs() < 1e-9);
        assert!(k < 0.0);
    }

    #[test]
    fn known_value() {
        // 10 items, agree on 8, each rater 6 "1" / 4 "0".
        let p = pairs(&[
            ("1", "1"), ("1", "1"), ("1", "1"), ("1", "1"), ("1", "1"),
            ("0", "0"), ("0", "0"), ("0", "0"), ("1", "0"), ("0", "1"),
        ]);
        let (k, po) = cohens_kappa(&p).unwrap();
        assert!((po - 0.8).abs() < 1e-9);
        // p_e = .6*.6 + .4*.4 = .52; kappa = (.8-.52)/(1-.52) = .5833…
        assert!((k - 0.5833333333333334).abs() < 1e-9, "kappa was {k}");
    }

    #[test]
    fn interpretation_bands() {
        assert_eq!(kappa_interpretation(-0.1), "Poor (worse than chance)");
        assert_eq!(kappa_interpretation(0.5), "Moderate");
        assert_eq!(kappa_interpretation(0.9), "Almost perfect");
    }
}
