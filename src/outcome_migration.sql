-- Migration: Add reason_code and is_user_labeled to session_outcomes
ALTER TABLE session_outcomes ADD COLUMN reason_code TEXT; -- 'repro_missing', 'context_drift', 'tool_misuse', 'dependency_trap', 'unknown'
ALTER TABLE session_outcomes ADD COLUMN is_user_labeled INTEGER DEFAULT 0;
