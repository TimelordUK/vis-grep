#[cfg(test)]
mod tests {
    use super::super::state::TreeFilter;
    
    #[test]
    fn test_tree_filter_matches() {
        let mut filter = TreeFilter::new();
        
        // Test cases for the word "test"
        let test_cases = vec![
            ("test", "test", true),
            ("test", "Test", true),
            ("test", "TEST", true),
            ("test", "test.log", true),
            ("test", "my_test_file.log", true),
            ("test", "/home/user/test_logs/file.log", true),
            ("test", "Test Log 1", true),
            ("test", "latest", true),  // Contains t-e-s-t in order
            ("test", "t e s t", true),
            ("test", "other", false),
            ("test", "set", false),  // Missing 't' at start
            ("test", "tse", false),  // Wrong order
        ];
        
        for (pattern, path, expected) in test_cases {
            filter.pattern = pattern.to_string();
            let result = filter.matches(path);
            println!("Pattern '{}' vs '{}': expected {}, got {}", pattern, path, expected, result);
            assert_eq!(result, expected, "Failed for pattern '{}' on path '{}'", pattern, path);
        }
    }
    
    #[test] 
    fn test_tree_filter_real_scenarios() {
        let mut filter = TreeFilter::new();
        
        // Simulate the test_tail_tree.sh scenario
        filter.pattern = "test".to_string();
        filter.active = true;
        
        let files = vec![
            ("/home/me/test_logs/test_1.log", "Test Log 1", true),
            ("/home/me/test_logs/test_2.log", "Test Log 2", true),
            ("/home/me/test_logs/test_3.log", "Test Log 3", true),
            ("Application Logs", "Application Logs", false),
            ("System Logs", "System Logs", false),
        ];
        
        for (path, name, expected) in files {
            let path_match = filter.matches(path);
            let name_match = filter.matches(name);
            let visible = path_match || name_match;
            
            println!("Filter 'test' on '{}' (name: '{}'): path={}, name={}, visible={}",
                     path, name, path_match, name_match, visible);
            
            assert_eq!(visible, expected, 
                      "Wrong visibility for path='{}', name='{}'", path, name);
        }
    }
}