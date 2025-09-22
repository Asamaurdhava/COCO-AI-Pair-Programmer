use std::collections::HashMap;
use std::path::Path;

use crate::app::{Thought, ThoughtType, Suggestion, ActionType, Priority};

pub struct CodeAnalyzer {
    language_patterns: HashMap<String, LanguageConfig>,
}

#[derive(Clone)]
struct LanguageConfig {
    file_extensions: Vec<String>,
    comment_patterns: Vec<String>,
    keywords: Vec<String>,
    common_patterns: Vec<Pattern>,
}

#[derive(Clone)]
struct Pattern {
    name: String,
    regex: String,
    severity: PatternSeverity,
    suggestion: String,
}

#[derive(Clone)]
enum PatternSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        let mut analyzer = Self {
            language_patterns: HashMap::new(),
        };

        analyzer.init_language_configs();
        analyzer
    }

    fn init_language_configs(&mut self) {
        // Rust configuration
        self.language_patterns.insert("rust".to_string(), LanguageConfig {
            file_extensions: vec!["rs".to_string()],
            comment_patterns: vec!["//".to_string(), "/*".to_string()],
            keywords: vec![
                "fn", "let", "mut", "const", "static", "struct", "enum", "impl", "trait",
                "mod", "use", "pub", "async", "await", "match", "if", "else", "for",
                "while", "loop", "return", "break", "continue"
            ].into_iter().map(|s| s.to_string()).collect(),
            common_patterns: vec![
                Pattern {
                    name: "unwrap_usage".to_string(),
                    regex: r"\.unwrap\(\)".to_string(),
                    severity: PatternSeverity::Warning,
                    suggestion: "Consider using proper error handling instead of .unwrap()".to_string(),
                },
                Pattern {
                    name: "clone_in_loop".to_string(),
                    regex: r"for\s+.*\{[^}]*\.clone\(\)".to_string(),
                    severity: PatternSeverity::Warning,
                    suggestion: "Avoid cloning in loops for better performance".to_string(),
                },
                Pattern {
                    name: "println_debug".to_string(),
                    regex: r"println!\s*\(".to_string(),
                    severity: PatternSeverity::Info,
                    suggestion: "Consider using logging instead of println! for production code".to_string(),
                },
            ],
        });

        // Python configuration
        self.language_patterns.insert("python".to_string(), LanguageConfig {
            file_extensions: vec!["py".to_string()],
            comment_patterns: vec!["#".to_string()],
            keywords: vec![
                "def", "class", "import", "from", "as", "if", "elif", "else", "for",
                "while", "try", "except", "finally", "with", "lambda", "return",
                "yield", "break", "continue", "pass", "global", "nonlocal"
            ].into_iter().map(|s| s.to_string()).collect(),
            common_patterns: vec![
                Pattern {
                    name: "bare_except".to_string(),
                    regex: r"except\s*:".to_string(),
                    severity: PatternSeverity::Warning,
                    suggestion: "Avoid bare except clauses; specify exception types".to_string(),
                },
                Pattern {
                    name: "print_statement".to_string(),
                    regex: r"print\s*\(".to_string(),
                    severity: PatternSeverity::Info,
                    suggestion: "Consider using logging instead of print for production code".to_string(),
                },
            ],
        });

        // JavaScript/TypeScript configuration
        self.language_patterns.insert("javascript".to_string(), LanguageConfig {
            file_extensions: vec!["js".to_string(), "ts".to_string(), "jsx".to_string(), "tsx".to_string()],
            comment_patterns: vec!["//".to_string(), "/*".to_string()],
            keywords: vec![
                "function", "var", "let", "const", "class", "extends", "implements",
                "interface", "if", "else", "for", "while", "do", "switch", "case",
                "return", "break", "continue", "try", "catch", "finally", "throw",
                "async", "await", "import", "export", "default"
            ].into_iter().map(|s| s.to_string()).collect(),
            common_patterns: vec![
                Pattern {
                    name: "console_log".to_string(),
                    regex: r"console\.log\s*\(".to_string(),
                    severity: PatternSeverity::Info,
                    suggestion: "Consider removing console.log statements in production code".to_string(),
                },
                Pattern {
                    name: "var_usage".to_string(),
                    regex: r"\bvar\s+".to_string(),
                    severity: PatternSeverity::Warning,
                    suggestion: "Consider using 'let' or 'const' instead of 'var'".to_string(),
                },
            ],
        });
    }

    pub fn detect_language(&self, file_path: &str) -> Option<String> {
        let path = Path::new(file_path);
        let extension = path.extension()?.to_str()?;

        for (lang, config) in &self.language_patterns {
            if config.file_extensions.contains(&extension.to_string()) {
                return Some(lang.clone());
            }
        }

        None
    }

    pub fn analyze_code_patterns(&self, code: &str, file_path: Option<&str>) -> Vec<Thought> {
        let mut thoughts = Vec::new();

        // Detect language
        let language = if let Some(path) = file_path {
            self.detect_language(path)
        } else {
            None
        };

        // Basic code metrics
        thoughts.extend(self.analyze_basic_metrics(code, file_path));

        // Language-specific analysis
        if let Some(lang) = language {
            if let Some(config) = self.language_patterns.get(&lang) {
                thoughts.extend(self.analyze_language_patterns(code, config, file_path));
            }
        }

        // General code quality analysis
        thoughts.extend(self.analyze_general_quality(code, file_path));

        thoughts
    }

    fn analyze_basic_metrics(&self, code: &str, file_path: Option<&str>) -> Vec<Thought> {
        let mut thoughts = Vec::new();

        let lines = code.lines().collect::<Vec<_>>();
        let line_count = lines.len();
        let _char_count = code.len();
        let _word_count = code.split_whitespace().count();

        // File size analysis
        if line_count > 500 {
            thoughts.push(Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                thought_type: ThoughtType::Warning,
                content: format!("Large file detected ({} lines). Consider breaking it into smaller modules.", line_count),
                file_path: file_path.map(|s| s.to_string()),
                line_number: None,
                confidence: 0.8,
                suggestions: vec![
                    Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: "Split large file".to_string(),
                        description: "Break this file into smaller, more focused modules".to_string(),
                        code_snippet: None,
                        action_type: ActionType::Refactor,
                        priority: Priority::Medium,
                    }
                ],
            });
        }

        // Long lines analysis
        let long_lines: Vec<_> = lines.iter().enumerate()
            .filter(|(_, line)| line.len() > 120)
            .collect();

        if !long_lines.is_empty() {
            thoughts.push(Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                thought_type: ThoughtType::Style,
                content: format!("Found {} lines longer than 120 characters. Consider breaking them up for better readability.", long_lines.len()),
                file_path: file_path.map(|s| s.to_string()),
                line_number: long_lines.first().map(|(i, _)| i + 1),
                confidence: 0.7,
                suggestions: vec![
                    Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: "Break long lines".to_string(),
                        description: "Split long lines to improve readability".to_string(),
                        code_snippet: None,
                        action_type: ActionType::Refactor,
                        priority: Priority::Low,
                    }
                ],
            });
        }

        // Comment density analysis
        let comment_lines = lines.iter()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*")
            })
            .count();

        let comment_ratio = if line_count > 0 {
            comment_lines as f32 / line_count as f32
        } else {
            0.0
        };

        if comment_ratio < 0.1 && line_count > 50 {
            thoughts.push(Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                thought_type: ThoughtType::Suggesting,
                content: format!("Low comment density ({:.1}%). Consider adding more documentation for complex logic.", comment_ratio * 100.0),
                file_path: file_path.map(|s| s.to_string()),
                line_number: None,
                confidence: 0.6,
                suggestions: vec![
                    Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: "Add documentation".to_string(),
                        description: "Add comments to explain complex logic and public APIs".to_string(),
                        code_snippet: None,
                        action_type: ActionType::Insert,
                        priority: Priority::Medium,
                    }
                ],
            });
        }

        thoughts
    }

    fn analyze_language_patterns(&self, code: &str, config: &LanguageConfig, file_path: Option<&str>) -> Vec<Thought> {
        let mut thoughts = Vec::new();

        for pattern in &config.common_patterns {
            if let Ok(regex) = regex::Regex::new(&pattern.regex) {
                let matches: Vec<_> = regex.find_iter(code).collect();

                if !matches.is_empty() {
                    let thought_type = match pattern.severity {
                        PatternSeverity::Info => ThoughtType::Analyzing,
                        PatternSeverity::Warning => ThoughtType::Warning,
                        PatternSeverity::Error => ThoughtType::Error,
                        PatternSeverity::Critical => ThoughtType::Error,
                    };

                    let confidence = match pattern.severity {
                        PatternSeverity::Critical => 0.95,
                        PatternSeverity::Error => 0.85,
                        PatternSeverity::Warning => 0.75,
                        PatternSeverity::Info => 0.6,
                    };

                    let priority = match pattern.severity {
                        PatternSeverity::Critical => Priority::Critical,
                        PatternSeverity::Error => Priority::High,
                        PatternSeverity::Warning => Priority::Medium,
                        PatternSeverity::Info => Priority::Low,
                    };

                    thoughts.push(Thought {
                        id: uuid::Uuid::new_v4().to_string(),
                        timestamp: chrono::Utc::now(),
                        thought_type,
                        content: format!("{} (found {} occurrences)", pattern.suggestion, matches.len()),
                        file_path: file_path.map(|s| s.to_string()),
                        line_number: None, // TODO: Calculate line number from match position
                        confidence,
                        suggestions: vec![
                            Suggestion {
                                id: uuid::Uuid::new_v4().to_string(),
                                title: pattern.name.replace('_', " ").to_string(),
                                description: pattern.suggestion.clone(),
                                code_snippet: None,
                                action_type: ActionType::Fix,
                                priority,
                            }
                        ],
                    });
                }
            }
        }

        thoughts
    }

    fn analyze_general_quality(&self, code: &str, file_path: Option<&str>) -> Vec<Thought> {
        let mut thoughts = Vec::new();

        // Complexity analysis (simplified)
        let nesting_level = self.calculate_max_nesting_level(code);
        if nesting_level > 4 {
            thoughts.push(Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                thought_type: ThoughtType::Warning,
                content: format!("High nesting level detected ({}). Consider refactoring to reduce complexity.", nesting_level),
                file_path: file_path.map(|s| s.to_string()),
                line_number: None,
                confidence: 0.8,
                suggestions: vec![
                    Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: "Reduce nesting".to_string(),
                        description: "Extract nested logic into separate functions or use early returns".to_string(),
                        code_snippet: None,
                        action_type: ActionType::Refactor,
                        priority: Priority::Medium,
                    }
                ],
            });
        }

        // Function length analysis (simplified)
        let long_functions = self.find_long_functions(code);
        if !long_functions.is_empty() {
            thoughts.push(Thought {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                thought_type: ThoughtType::Suggesting,
                content: format!("Found {} potentially long functions. Consider breaking them into smaller, focused functions.", long_functions.len()),
                file_path: file_path.map(|s| s.to_string()),
                line_number: None,
                confidence: 0.7,
                suggestions: vec![
                    Suggestion {
                        id: uuid::Uuid::new_v4().to_string(),
                        title: "Split long functions".to_string(),
                        description: "Break large functions into smaller, single-purpose functions".to_string(),
                        code_snippet: None,
                        action_type: ActionType::Refactor,
                        priority: Priority::Low,
                    }
                ],
            });
        }

        // TODO: Add more sophisticated analysis
        // - Cyclomatic complexity
        // - Code duplication detection
        // - Performance anti-patterns
        // - Security vulnerabilities

        thoughts
    }

    fn calculate_max_nesting_level(&self, code: &str) -> usize {
        let mut max_level = 0;
        let mut current_level = 0;

        for ch in code.chars() {
            match ch {
                '{' | '(' | '[' => {
                    current_level += 1;
                    max_level = max_level.max(current_level);
                }
                '}' | ')' | ']' => {
                    if current_level > 0 {
                        current_level -= 1;
                    }
                }
                _ => {}
            }
        }

        max_level
    }

    fn find_long_functions(&self, code: &str) -> Vec<String> {
        // Simplified function detection - this would need to be more sophisticated for real use
        let mut functions = Vec::new();
        let lines: Vec<&str> = code.lines().collect();

        let mut in_function = false;
        let mut function_start = 0;
        let mut brace_count = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Simple function detection (very basic)
            if (trimmed.contains("fn ") || trimmed.contains("function ") || trimmed.contains("def "))
                && trimmed.contains("(") {
                in_function = true;
                function_start = i;
                brace_count = 0;
            }

            if in_function {
                brace_count += line.matches('{').count() as i32;
                brace_count -= line.matches('}').count() as i32;

                if brace_count <= 0 && i > function_start {
                    let function_length = i - function_start + 1;
                    if function_length > 50 {  // Consider functions longer than 50 lines as long
                        functions.push(format!("Function at line {} ({} lines)", function_start + 1, function_length));
                    }
                    in_function = false;
                }
            }
        }

        functions
    }

    pub fn generate_summary(&self, thoughts: &[Thought]) -> String {
        if thoughts.is_empty() {
            return "No issues found. Code looks good!".to_string();
        }

        let mut summary = String::new();

        let error_count = thoughts.iter().filter(|t| matches!(t.thought_type, ThoughtType::Error)).count();
        let warning_count = thoughts.iter().filter(|t| matches!(t.thought_type, ThoughtType::Warning)).count();
        let suggestion_count = thoughts.iter().filter(|t| matches!(t.thought_type, ThoughtType::Suggesting)).count();

        summary.push_str(&format!("Analysis complete: {} thoughts generated\n", thoughts.len()));

        if error_count > 0 {
            summary.push_str(&format!("🔴 {} errors found\n", error_count));
        }

        if warning_count > 0 {
            summary.push_str(&format!("⚠️ {} warnings found\n", warning_count));
        }

        if suggestion_count > 0 {
            summary.push_str(&format!("💡 {} suggestions available\n", suggestion_count));
        }

        // Calculate average confidence
        let avg_confidence = if !thoughts.is_empty() {
            thoughts.iter().map(|t| t.confidence).sum::<f32>() / thoughts.len() as f32
        } else {
            0.0
        };

        summary.push_str(&format!("📊 Average confidence: {:.1}%", avg_confidence * 100.0));

        summary
    }
}