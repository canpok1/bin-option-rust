ALTER TABLE binopt.forecast_models ADD feature_params JSON COMMENT '特徴量用パラメータ' AFTER input_data_size;
ALTER TABLE binopt.forecast_models ADD feature_params_hash TEXT NOT NULL COMMENT '特徴量用パラメータのハッシュ値' AFTER feature_params;
UPDATE forecast_models SET feature_params = JSON_OBJECT() WHERE feature_params IS NULL;
ALTER TABLE binopt.forecast_models MODIFY COLUMN feature_params JSON NOT NULL;
