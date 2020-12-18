FROM rust

RUN echo "yo"

RUN ls

WORKDIR /home

RUN ls

COPY . .

RUN ls

CMD ./runeverything.sh
