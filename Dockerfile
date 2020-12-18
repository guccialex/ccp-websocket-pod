FROM rust

WORKDIR /home

COPY . .

CMD ./runeverything.sh
