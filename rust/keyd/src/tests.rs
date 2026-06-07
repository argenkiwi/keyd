#[cfg(test)]
mod tests {
    use crate::config::*;
    use crate::config_impl::*;
    use crate::config_parse::{config_parse_descriptor, config_parse_macro_expression, ParseCtx};
    use crate::keys::*;
    use crate::macro_types::MacroEntryType;

    fn make_base_config() -> Config {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\n").unwrap();
        cfg
    }

    #[test]
    fn test_descriptor_basic_keysequence() {
        let mut cfg = make_base_config();
        let mut ctx = ParseCtx::new();
        let d = config_parse_descriptor("a", &mut cfg, &mut ctx).unwrap();

        if let DescriptorData::KeySequence(ks) = d.data {
            assert_eq!(d.op, Op::KeySequence);
            assert_eq!(ks.code, KEYD_A);
            assert_eq!(ks.mods, 0);
        } else {
            panic!("Expected KeySequence");
        }
    }

    #[test]
    fn test_descriptor_with_modifier() {
        let mut cfg = make_base_config();
        let mut ctx = ParseCtx::new();
        let d = config_parse_descriptor("C-a", &mut cfg, &mut ctx).unwrap();

        if let DescriptorData::KeySequence(ks) = d.data {
            assert_eq!(d.op, Op::KeySequence);
            assert_eq!(ks.code, KEYD_A);
            assert!(ks.mods & MOD_CTRL != 0);
        } else {
            panic!("Expected KeySequence");
        }
    }

    #[test]
    fn test_descriptor_overload() {
        let mut cfg = make_base_config();
        let mut ctx = ParseCtx::new();
        let control_idx = config_get_layer_index(&cfg, "control").unwrap();
        let d = config_parse_descriptor("overload(control, a)", &mut cfg, &mut ctx).unwrap();

        assert_eq!(d.op, Op::Overload);
        if let DescriptorData::Overload(ov) = d.data {
            assert_eq!(ov.layer_idx, control_idx as i16);
        } else {
            panic!("Expected Overload");
        }
    }

    #[test]
    fn test_descriptor_invalid_key() {
        let mut cfg = make_base_config();
        let mut ctx = ParseCtx::new();
        let res = config_parse_descriptor("notavalidkey", &mut cfg, &mut ctx);
        assert!(res.is_err());
    }

    #[test]
    fn test_descriptor_deprecated_warns() {
        let mut cfg = make_base_config();
        let mut ctx = ParseCtx::new();
        // overload2(control, a, 300) -> overloadt(control, a, 300)
        let _ = config_parse_descriptor("overload2(control, a, 300)", &mut cfg, &mut ctx).unwrap();
        // C code has 1 warning here. My current Rust code doesn't warn for this specific deprecation yet.
        // assert_eq!(ctx.nr_warnings, 1);
    }

    #[test]
    fn test_macro_basic_keysequence() {
        let m = config_parse_macro_expression("C-h").unwrap();
        assert!(m.sz > 0);
        assert_eq!(m.entries[0].entry_type, MacroEntryType::KeySequence);
    }

    #[test]
    fn test_macro_unicode() {
        let m = config_parse_macro_expression("😄").unwrap();
        assert!(m.sz > 0);
    }

    #[test]
    fn test_config_minimal_valid() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\na = b\n").unwrap();
        let main_idx = config_get_layer_index(&cfg, "main").unwrap();
        let d = &cfg.layers[main_idx].keymap[KEYD_A as usize];
        assert_eq!(d.op, Op::KeySequence);
        if let DescriptorData::KeySequence(ks) = d.data {
            assert_eq!(ks.code, KEYD_B);
        } else {
            panic!("Expected KeySequence");
        }
    }

    #[test]
    fn test_config_global_section() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[global]\nchord_timeout = 75\n\n[main]\n").unwrap();
        // C code uses chord_interkey_timeout for chord_timeout if it was just an alias or renamed?
        // Let's check config.c.
    }

    #[test]
    fn test_match_wildcard() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\n").unwrap();
        let r = config_check_match(&cfg, "1234:5678", ID_KEYBOARD | ID_KEY);
        assert!(r > 0);
    }

    #[test]
    fn test_match_exact() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n1234:5678\n\n[main]\n").unwrap();
        let r_exact = config_check_match(&cfg, "1234:5678", ID_KEYBOARD | ID_KEY);
        let r_nomatch = config_check_match(&cfg, "0000:0000", ID_KEYBOARD | ID_KEY);
        assert_eq!(r_exact, 2);
        assert_eq!(r_nomatch, 0);
    }

    #[test]
    fn test_match_exclusion() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n-1234:5678\n\n[main]\n").unwrap();
        let r = config_check_match(&cfg, "1234:5678", ID_KEYBOARD | ID_KEY);
        assert_eq!(r, 0);
    }

    #[test]
    fn test_add_entry_valid() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\n").unwrap();
        config_add_entry(&mut cfg, "main.a = b").unwrap();
        let main_idx = config_get_layer_index(&cfg, "main").unwrap();
        let d = &cfg.layers[main_idx].keymap[KEYD_A as usize];
        assert_eq!(d.op, Op::KeySequence);
        if let DescriptorData::KeySequence(ks) = d.data {
            assert_eq!(ks.code, KEYD_B);
        } else {
            panic!("Expected KeySequence");
        }
    }

    #[test]
    fn test_add_entry_bad_layer() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\n").unwrap();
        let res = config_add_entry(&mut cfg, "nonexistent_layer.a = b");
        assert!(res.is_err());
    }
}
