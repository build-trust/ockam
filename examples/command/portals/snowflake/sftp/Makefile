.PHONY: all
all: setup service inlet

.PHONY: setup
setup:
	snow sql --filename setup.sql --variable egress_host_port=$(shell ockam project show --jq '.egress_allow_list[]')

.PHONY: compute_pool_status
compute_pool_status:
	snow sql -q "DESCRIBE COMPUTE POOL sftp_compute_pool" --role sftp_role

.PHONY: containers
service: images
	snow sql --filename service.sql \
		--variable ticket=$(shell ockam project ticket --usage-count 1 --expires-in 1h --attribute sftp-outlet --relay sftp-relay)

.PHONY: inlet
inlet:
	docker rmi ghcr.io/build-trust/ockam || true
	docker run --rm -d --name sftp-inlet -p 2222:2222 ghcr.io/build-trust/ockam node create --foreground \
		--enrollment-ticket "$(shell ockam project ticket --usage-count 1 --expires-in 1h --attribute sftp-inlet)" \
		--configuration '{"tcp-inlet":{"from":"0.0.0.0:2222","via":"sftp-relay","allow":"sftp-outlet"}}'

.PHONY: test
test:
	echo 'hello' > hello.txt
	echo 'put hello.txt /stage/hello.txt' | sftp -i ./sftp_host_rsa_key -o UserKnownHostsFile=/dev/null -P 2222 foo@127.0.0.1
	echo 'get /stage/hello.txt hello1.txt' | sftp -i ./sftp_host_rsa_key -o UserKnownHostsFile=/dev/null -P 2222 foo@127.0.0.1
	if cmp -s hello.txt hello1.txt; then echo 'PASSED'; else echo 'FAILED'; exit 1; fi
	rm hello.txt hello1.txt

.PHONY: image_repository
image_repository:
	snow spcs image-registry login --role sftp_role
	$(eval REPO := $(shell snow spcs image-repository url \
	   sftp_database.sftp_schema.sftp_image_repository \
	   --role sftp_role \
	   --warehouse sftp_warehouse))

.PHONY: ockam_image
ockam_image: image_repository
	docker rmi ghcr.io/build-trust/ockam || true
	docker pull --platform=linux/amd64 ghcr.io/build-trust/ockam
	docker tag ghcr.io/build-trust/ockam $(REPO)/ockam
	docker push $(REPO)/ockam

.PHONY: sftp_image
sftp_image: image_repository
	ssh-keygen -b 4096 -t rsa -f ./sftp_host_rsa_key -N ''
	docker build --platform linux/amd64 -t $(REPO)/sftp -f sftp_server.Dockerfile .
	docker push $(REPO)/sftp

.PHONY: images
images: sftp_image ockam_image

.PHONY: cleanup
cleanup:
	docker rm -f sftp-inlet
	snow sql --filename cleanup.sql
