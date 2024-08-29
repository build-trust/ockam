.PHONY: all
all: setup service inlet

.PHONY: setup
setup:
	snow sql --filename setup.sql --variable egress_host_port=$(shell ockam project show --jq '.egress_allow_list[]')

.PHONY: compute_pool_status
compute_pool_status:
	snow sql -q "DESCRIBE COMPUTE POOL webdav_compute_pool" --role webdav_role

.PHONY: containers
service: images
	snow sql --filename service.sql \
		--variable ticket=$(shell ockam project ticket --usage-count 1 --expires-in 1h --attribute webdav-outlet --relay webdav-relay)

.PHONY: inlet
inlet:
	docker rmi ghcr.io/build-trust/ockam || true
	docker run --rm -d --name webdav-inlet -p 8001:8001 ghcr.io/build-trust/ockam node create --foreground \
		--enrollment-ticket "$(shell ockam project ticket --usage-count 1 --expires-in 1h --attribute webdav-inlet)" \
		--configuration '{"tcp-inlet":{"from":"0.0.0.0:8001","via":"webdav-relay","allow":"webdav-outlet"}}'

.PHONY: test
test:
	curl --head http://localhost:8001

.PHONY: image_repository
image_repository:
	snow spcs image-registry login --role webdav_role
	$(eval REPO := $(shell snow spcs image-repository url \
	   webdav_database.webdav_schema.webdav_image_repository \
	   --role webdav_role \
	   --warehouse webdav_warehouse))

.PHONY: ockam_image
ockam_image: image_repository
	docker rmi ghcr.io/build-trust/ockam || true
	docker pull --platform=linux/amd64 ghcr.io/build-trust/ockam
	docker tag ghcr.io/build-trust/ockam $(REPO)/ockam
	docker push $(REPO)/ockam

.PHONY: webdav_image
webdav_image: image_repository
	docker rmi bytemark/webdav || true
	docker pull --platform=linux/amd64 bytemark/webdav
	docker tag bytemark/webdav $(REPO)/webdav
	docker push $(REPO)/webdav

.PHONY: images
images: webdav_image ockam_image

.PHONY: cleanup
cleanup:
	docker rm -f webdav-inlet
	snow sql --filename cleanup.sql
