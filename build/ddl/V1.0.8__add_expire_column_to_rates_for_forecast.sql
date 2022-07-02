ALTER TABLE binopt.rates_for_forecast ADD expire DATETIME NOT NULL COMMENT '有効期限' AFTER histories;
