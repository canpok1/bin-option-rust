ALTER TABLE binopt.forecast_models ADD performance_rmse DOUBLE UNSIGNED NOT NULL DEFAULT 1.0 COMMENT 'パフォーマンス（二乗平均平方根誤差）' AFTER performance_mse;
