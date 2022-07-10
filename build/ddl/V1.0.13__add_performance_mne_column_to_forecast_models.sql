ALTER TABLE binopt.forecast_models ADD performance_mse DOUBLE UNSIGNED NOT NULL DEFAULT 1.0 COMMENT 'パフォーマンス（平均二乗誤差）' AFTER feature_params_hash;
