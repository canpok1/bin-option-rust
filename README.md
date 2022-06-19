# bin-option-rust

## 使い方

### ホスト上

```
# DB起動
# VSCode Remote Container使用時は自動起動するため本コマンドは不要
docker-compose -f ./build/docker-compose-db.yml up -d db

# DBのmigrate
# 接続情報は local.env のものを使用
docker-compose up flyway
```

### 開発コンテナ上

```
# 操作は下記いずれかを参照
makers --list-category-steps MyCommand
cargo make --list-category-steps MyCommand
```
