.PHONY: build test vet install
build:
	CGO_ENABLED=0 GOTOOLCHAIN=local GOWORK=off go build -trimpath -buildvcs=false -o dist/gw ./cmd/gw

test:
	GOTOOLCHAIN=local GOWORK=off go test ./...

vet:
	GOTOOLCHAIN=local GOWORK=off go vet ./...

install: build
	./gw install
