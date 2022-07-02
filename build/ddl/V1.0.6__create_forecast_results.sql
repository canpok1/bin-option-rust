CREATE TABLE forecast_results (
    id CHAR(36) NOT NULL DEFAULT (UUID()) COMMENT 'ID',
    rate_id CHAR(36) NOT NULL COMMENT '予測用のレートID',
    forecast_type TINYINT UNSIGNED NOT NULL COMMENT '予測種別',
    result DECIMAL(15,4) NOT NULL COMMENT '予測結果',
    memo TEXT COMMENT 'メモ',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '作成日時',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新日時',
    PRIMARY KEY(id),
    FOREIGN KEY fk_rate_id(rate_id) REFERENCES rates_for_forecast(id)
)
COMMENT='予測結果'
;

