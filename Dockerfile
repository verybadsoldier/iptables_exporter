FROM alpine:3.20.3 AS builder

RUN apk add --no-cache cargo git rust

COPY . /src

RUN cd /src && cargo build -r

FROM builder

COPY --from=builder /src/target/release/iptables_exporter /iptables_exporter/iptables_exporter

RUN apk add --no-cache wget bash iptables iptables-legacy libcap ipset
   

# set file capabilities so container can be used by non-root user
RUN for f in /usr/sbin/ipset /sbin/xtables-nft-multi /sbin/xtables-legacy-multi; do setcap cap_net_admin,cap_net_raw,cap_dac_read_search+eip "${f}"; done
	
ENTRYPOINT ["/iptables_exporter/iptables_exporter"]

    
