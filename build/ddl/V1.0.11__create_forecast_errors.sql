CREATE TABLE forecast_errors (
    id CHAR(36) NOT NULL DEFAULT (UUID()) COMMENT 'ID',
    rate_id CHAR(36) NOT NULL COMMENT '予測用のレートID',
    model_no INTEGER NOT NULL COMMENT '予測を行ったモデルのモデルNo',
    summary TEXT NOT NULL COMMENT 'エラー概要',
    detail TEXT NOT NULL COMMENT 'エラー詳細',
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT '作成日時',
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT '更新日時',
    PRIMARY KEY(id),
    FOREIGN KEY fk_forecast_errors_rate_id(rate_id) REFERENCES rates_for_forecast(id)
)
COMMENT='予測エラー'
;

