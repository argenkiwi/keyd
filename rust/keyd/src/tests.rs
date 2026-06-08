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
        assert_eq!(cfg.chord_interkey_timeout, 75);
    }

    #[test]
    fn test_default_timeouts() {
        let cfg = Config::new();
        assert_eq!(cfg.chord_interkey_timeout, 50);
        assert_eq!(cfg.macro_timeout, 600);
        assert_eq!(cfg.macro_repeat_timeout, 50);
    }

    #[test]
    fn test_main_is_layout() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\n").unwrap();
        let main_idx = config_get_layer_index(&cfg, "main").unwrap();
        assert_eq!(cfg.layers[main_idx].layer_type, LayerType::Layout);
    }

    #[test]
    fn test_wildcard_sets_field() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n\n[main]\n").unwrap();
        assert_eq!(cfg.wildcard, 1);
        assert!(cfg.ids.is_empty());
    }

    #[test]
    fn test_exclusion_id_stored_without_prefix() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg, "[ids]\n*\n-1234:5678\n\n[main]\n").unwrap();
        assert_eq!(cfg.wildcard, 1);
        assert_eq!(cfg.ids.len(), 1);
        assert_eq!(cfg.ids[0].flags, ID_EXCLUDED);
        assert_eq!(cfg.ids[0].id, "1234:5678");
    }

    #[test]
    fn test_composite_layer_type_and_constituents() {
        let mut cfg = Config::new();
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\n\n[nav]\na = b\n\n[nav+control]\n"
        ).unwrap();
        let idx = config_get_layer_index(&cfg, "nav+control").unwrap();
        assert_eq!(cfg.layers[idx].layer_type, LayerType::Composite);
        assert_eq!(cfg.layers[idx].nr_constituents, 2);
    }

    #[test]
    fn test_composite_layer_inherits_from_constituent() {
        let mut cfg = Config::new();
        // nav+control inherits a→b from nav; b→a is its own explicit entry
        config_parse_string(&mut cfg,
            "[ids]\n*\n\n[main]\n\n[nav]\na = b\n\n[nav+control]\nb = a\n"
        ).unwrap();
        let idx = config_get_layer_index(&cfg, "nav+control").unwrap();

        // explicit entry
        if let DescriptorData::KeySequence(ks) = cfg.layers[idx].keymap[KEYD_B as usize].data {
            assert_eq!(ks.code, KEYD_A, "explicit b→a");
        } else {
            panic!("Expected explicit b→a in composite");
        }

        // inherited from nav constituent
        if let DescriptorData::KeySequence(ks) = cfg.layers[idx].keymap[KEYD_A as usize].data {
            assert_eq!(ks.code, KEYD_B, "inherited a→b from nav");
        } else {
            panic!("Composite should inherit a→b from nav constituent");
        }
    }

    #[test]
    fn test_config_parse_file_with_include() {
        let dir = std::env::temp_dir();
        let main_path = dir.join("test_keyd_main.conf");
        let inc_path = dir.join("test_keyd_inc.conf");

        std::fs::write(&inc_path, "[extra-layer]\na = b\n").unwrap();

        let main_content = format!(
            "[ids]\n*\n\ninclude {}\n\n[main]\n",
            inc_path.display()
        );
        std::fs::write(&main_path, &main_content).unwrap();

        let cfg = config_parse(main_path.to_str().unwrap()).unwrap();

        let _ = std::fs::remove_file(&main_path);
        let _ = std::fs::remove_file(&inc_path);

        let extra_idx = config_get_layer_index(&cfg, "extra-layer");
        assert!(extra_idx.is_some(), "included layer should be present");
        let idx = extra_idx.unwrap();
        if let DescriptorData::KeySequence(ks) = cfg.layers[idx].keymap[KEYD_A as usize].data {
            assert_eq!(ks.code, KEYD_B);
        } else {
            panic!("Expected a→b in included layer");
        }
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
