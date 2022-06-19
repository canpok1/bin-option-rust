FROM rust:latest

RUN cargo install --force cargo-make

RUN apt update && apt install -y \
    maven \
    jq \
    && apt clean

RUN curl https://raw.githubusercontent.com/OpenAPITools/openapi-generator/master/bin/utils/openapi-generator-cli.sh > /usr/local/bin/openapi-generator-cli \
    && chmod u+x /usr/local/bin/openapi-generator-cli \
    && openapi-generator-cli version
