ALTER TABLE binopt.forecast_results ADD model_no INTEGER NOT NULL COMMENT '予測を行ったモデルのモデルNo' AFTER rate_id;
