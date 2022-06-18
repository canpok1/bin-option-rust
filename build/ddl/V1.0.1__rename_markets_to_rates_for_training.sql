ALTER TABLE markets RENAME rates_for_training;
ALTER TABLE rates_for_training COMMENT '学習用のレート情報';
