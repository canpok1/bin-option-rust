ALTER TABLE binopt.forecast_models ADD input_data_size INTEGER UNSIGNED COMMENT '入力データ数' AFTER model_data;
UPDATE forecast_models SET input_data_size = 50 WHERE input_data_size IS NULL;
ALTER TABLE binopt.forecast_models MODIFY COLUMN input_data_size INTEGER UNSIGNED NOT NULL;
