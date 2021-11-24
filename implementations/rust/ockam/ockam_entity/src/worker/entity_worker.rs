use crate::{
    EntityError::IdentityApiFailed, IdentityRequest, IdentityRequest::*, IdentityResponse as Res,
    MaybeContact, Profile, ProfileChannelListener, ProfileIdentifier, ProfileState,
    SecureChannelWorker, TrustPolicyImpl,
};
use core::result::Result::Ok;
use ockam_core::{
    async_trait::async_trait, compat::boxed::Box, compat::collections::HashMap, Address, Result,
    Routed, Worker,
};
use ockam_node::{Context, Handle};
use ockam_vault_sync_core::VaultSync;

#[cfg(feature = "lease_proto_json")]
use crate::lease::json_proto::{LeaseProtocolRequest, LeaseProtocolResponse};

#[derive(Default)]
pub struct EntityWorker {
    profiles: HashMap<ProfileIdentifier, ProfileState>,
}

impl EntityWorker {
    fn profile(&mut self, profile_id: &ProfileIdentifier) -> &mut ProfileState {
        self.profiles
            .get_mut(profile_id)
            .expect("default profile invalid")
    }
}

fn err<T>() -> Result<T> {
    Err(IdentityApiFailed.into())
}

#[async_trait]
impl Worker for EntityWorker {
    type Message = IdentityRequest;
    type Context = Context;

    async fn handle_message(
        &mut self,
        ctx: &mut Self::Context,
        msg: Routed<Self::Message>,
    ) -> Result<()> {
        let reply = msg.return_route();
        let req = msg.body();
        match req {
            CreateProfile(vault_address) => {
                let vault_sync = VaultSync::create_with_worker(ctx, &vault_address)
                    .await
                    .expect("couldn't create profile vault");

                let profile_state = ProfileState::create(vault_sync)
                    .await
                    .expect("failed to create ProfileState");

                let id = profile_state
                    .identifier()
                    .await
                    .expect("failed to get profile id");

                self.add_profile_state(profile_state)
                    .await
                    .expect("failed to add profile state");

                ctx.send(reply, Res::CreateProfile(id)).await
            }
            RemoveProfile(profile_id) => self.remove_profile(profile_id),
            CreateKey(profile_id, label) => {
                let profile = self.profile(&profile_id);

                profile.create_key(label).await
            }
            AddKey(profile_id, label, secret) => {
                let profile = self.profile(&profile_id);

                profile.add_key(label, &secret).await?;

                ctx.send(reply, Res::AddKey).await
            }
            RotateKey(profile_id) => {
                let profile = self.profile(&profile_id);

                profile.rotate_root_secret_key().await
            }
            GetProfilePublicKey(profile_id) => {
                if let Ok(public_key) = self.profile(&profile_id).get_root_public_key().await {
                    ctx.send(reply, Res::GetProfilePublicKey(public_key)).await
                } else {
                    err()
                }
            }
            GetProfileSecretKey(profile_id) => {
                if let Ok(secret) = self.profile(&profile_id).get_root_secret_key().await {
                    ctx.send(reply, Res::GetProfileSecretKey(secret)).await
                } else {
                    err()
                }
            }
            GetPublicKey(profile_id, label) => {
                if let Ok(public_key) = self.profile(&profile_id).get_public_key(label).await {
                    ctx.send(reply, Res::GetPublicKey(public_key)).await
                } else {
                    err()
                }
            }
            GetSecretKey(profile_id, label) => {
                if let Ok(secret) = self.profile(&profile_id).get_secret_key(label).await {
                    ctx.send(reply, Res::GetSecretKey(secret)).await
                } else {
                    err()
                }
            }
            CreateAuthenticationProof(profile_id, state) => {
                if let Ok(proof) = self
                    .profile(&profile_id)
                    .create_auth_proof(state.as_slice())
                    .await
                {
                    ctx.send(reply, Res::CreateAuthenticationProof(proof)).await
                } else {
                    err()
                }
            }
            VerifyAuthenticationProof(profile_id, state, peer_id, proof) => {
                if let Ok(verified) = self
                    .profile(&profile_id)
                    .verify_auth_proof(state.as_slice(), &peer_id, proof.as_slice())
                    .await
                {
                    ctx.send(reply, Res::VerifyAuthenticationProof(verified))
                        .await
                } else {
                    err()
                }
            }
            AddChange(profile_id, change) => self.profile(&profile_id).add_change(change).await,
            GetChanges(profile_id) => {
                let changes = self
                    .profile(&profile_id)
                    .get_changes()
                    .await
                    .expect("get_changes failed");
                ctx.send(reply, Res::GetChanges(changes)).await
            }
            VerifyChanges(profile_id) => {
                let verified = self.profile(&profile_id).verify_changes().await?;
                ctx.send(reply, Res::VerifyChanges(verified)).await
            }
            VerifyAndAddContact(profile_id, contact_id) => {
                let verified_and_added = self
                    .profile(&profile_id)
                    .verify_and_add_contact(contact_id)
                    .await?;
                ctx.send(reply, Res::VerifyAndAddContact(verified_and_added))
                    .await
            }
            GetContacts(profile_id) => {
                let contacts = self.profile(&profile_id).get_contacts().await?;
                ctx.send(reply, Res::Contacts(contacts)).await
            }
            VerifyContact(profile_id, contact) => {
                let verified = self.profile(&profile_id).verify_contact(contact).await?;
                ctx.send(reply, Res::VerifyContact(verified)).await
            }
            VerifyAndUpdateContact(profile_id, contact_id, changes) => {
                let verified = self
                    .profile(&profile_id)
                    .verify_and_update_contact(&contact_id, &changes)
                    .await?;
                ctx.send(reply, Res::VerifyAndUpdateContact(verified)).await
            }
            GetContact(profile_id, contact_id) => {
                let contact = self.profile(&profile_id).get_contact(&contact_id).await?;
                let message = match contact {
                    None => MaybeContact::None,
                    Some(contact) => MaybeContact::Contact(contact),
                };
                ctx.send(reply, Res::GetContact(message)).await
            }
            CreateSecureChannelListener(profile_id, address, trust_policy_address) => {
                let trust_policy = TrustPolicyImpl::new(Handle::new(
                    ctx.new_context(Address::random(0)).await?,
                    trust_policy_address,
                ));
                let vault_address = self.profile(&profile_id).vault_address();
                let handle = Handle::new(ctx.new_context(Address::random(0)).await?, ctx.address());
                let profile = Profile::new(profile_id, handle);
                let vault = VaultSync::create_with_worker(ctx, &vault_address).await?;
                let listener = ProfileChannelListener::new(trust_policy, profile, vault);
                ctx.start_worker(address, listener).await?;
                ctx.send(reply, Res::CreateSecureChannelListener).await
            }
            CreateSecureChannel(profile_id, route, trust_policy_address) => {
                let trust_policy = TrustPolicyImpl::new(Handle::new(
                    ctx.new_context(Address::random(0)).await?,
                    trust_policy_address,
                ));
                let vault_address = self.profile(&profile_id).vault_address();
                let handle = Handle::new(ctx.new_context(Address::random(0)).await?, ctx.address());
                let profile = Profile::new(profile_id.clone(), handle);

                let child_ctx = ctx.new_context(Address::random(0)).await?;
                let rt = ctx.runtime();
                rt.spawn(async move {
                    let vault = VaultSync::create_with_worker(&child_ctx, &vault_address).await?;
                    let address = SecureChannelWorker::create_initiator(
                        &child_ctx,
                        route,
                        profile,
                        trust_policy,
                        vault,
                    )
                    .await?;
                    child_ctx
                        .send(reply, Res::CreateSecureChannel(address))
                        .await
                });

                Ok(())
            }
            GetLease(lease_manager_route, profile_id, org_id, bucket, ttl) => {
                let profile = self.profile(&profile_id);
                if let Ok(lease) = profile
                    .get_lease(&lease_manager_route, org_id.clone(), bucket.clone(), ttl)
                    .await
                {
                    ctx.send(reply, Res::Lease(lease)).await
                } else {
                    #[cfg(feature = "lease_proto_json")]
                    {
                        // Send service request
                        let json = LeaseProtocolRequest::create(ttl, org_id, bucket).as_json();
                        ctx.send(lease_manager_route.clone(), json).await?;

                        // Wait for the response from the service
                        let json = ctx.receive::<String>().await?;

                        let lease_response = LeaseProtocolResponse::from_json(json.as_str());
                        if lease_response.is_success() {
                            ctx.send(reply.clone(), Res::Lease(lease_response.lease()))
                                .await
                        } else {
                            tracing::error!("Failed to get a lease from the lease manager");
                            err()
                        }
                    }
                    #[cfg(not(feature = "lease_proto_json"))]
                    panic!("No lease protocol implementations available")
                }
            }
            RevokeLease(lease_manager_route, profile_id, lease) => {
                self.profile(&profile_id)
                    .revoke_lease(&lease_manager_route, lease)
                    .await
            }
            #[cfg(feature = "credentials")]
            CredentialRequest(req) => {
                use crate::IdentityCredentialRequest::*;
                use crate::IdentityCredentialResponse as CredRes;
                use crate::{
                    CredentialHolder, CredentialIssuer, CredentialProof, CredentialPublicKey,
                    CredentialRequestFragment, CredentialVerifier,
                };
                match req {
                    GetSigningKey(profile_id) => {
                        if let Ok(signing_key) = self.profile(&profile_id).get_signing_key().await {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::GetSigningKey(signing_key)),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    GetIssuerPublicKey(profile_id) => {
                        if let Ok(public_key) =
                            self.profile(&profile_id).get_signing_public_key().await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::GetIssuerPublicKey(
                                    CredentialPublicKey(public_key),
                                )),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    CreateOffer(profile_id, schema) => {
                        if let Ok(offer) = self.profile(&profile_id).create_offer(&schema).await {
                            ctx.send(reply, Res::CredentialResponse(CredRes::CreateOffer(offer)))
                                .await
                        } else {
                            err()
                        }
                    }
                    CreateProofOfPossession(profile_id) => {
                        if let Ok(pop) =
                            self.profile(&profile_id).create_proof_of_possession().await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::CreateProofOfPossession(
                                    CredentialProof(pop),
                                )),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    SignCredential(profile_id, schema, attributes) => {
                        if let Ok(credential) = self
                            .profile(&profile_id)
                            .sign_credential(&schema, attributes.as_slice())
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::SignCredential(credential)),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    SignCredentialRequest(profile_id, request, schema, attributes, offer_id) => {
                        if let Ok(frag) = self
                            .profile(&profile_id)
                            .sign_credential_request(
                                &request,
                                &schema,
                                attributes.as_slice(),
                                offer_id,
                            )
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::SignCredentialRequest(frag)),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    AcceptCredentialOffer(profile_id, offer, signing_public_key) => {
                        if let Ok(cred_and_fragment) = self
                            .profile(&profile_id)
                            .accept_credential_offer(&offer, signing_public_key.0)
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::AcceptCredentialOffer(
                                    CredentialRequestFragment(
                                        cred_and_fragment.0,
                                        cred_and_fragment.1,
                                    ),
                                )),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    CombineCredentialFragments(profile_id, frag1, frag2) => {
                        if let Ok(credential) = self
                            .profile(&profile_id)
                            .combine_credential_fragments(frag1, frag2)
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::CombineCredentialFragments(
                                    credential,
                                )),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    IsValidCredential(profile_id, credential, issuer_public_key) => {
                        if let Ok(valid) = self
                            .profile(&profile_id)
                            .is_valid_credential(&credential, issuer_public_key.0)
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::IsValidCredential(valid)),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    PresentCredential(profile_id, credential, manifest, request_id) => {
                        if let Ok(presentations) = self
                            .profile(&profile_id)
                            .present_credentials(&[credential], &[manifest], request_id)
                            .await
                        {
                            let presentation = presentations
                                .first()
                                .expect("expected at least one presentation");

                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::PresentCredential(
                                    presentation.clone(),
                                )),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    CreateProofRequestId(profile_id) => {
                        if let Ok(request_id) =
                            self.profile(&profile_id).create_proof_request_id().await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::CreateProofRequestId(request_id)),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    VerifyProofOfPossession(
                        profile_id,
                        signing_public_key,
                        proof_of_possession,
                    ) => {
                        if let Ok(valid) = self
                            .profile(&profile_id)
                            .verify_proof_of_possession(signing_public_key.0, proof_of_possession.0)
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::VerifyProofOfPossession(valid)),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    VerifyCredentialPresentation(
                        profile_id,
                        presentation,
                        manifest,
                        request_id,
                    ) => {
                        if let Ok(valid) = self
                            .profile(&profile_id)
                            .verify_credential_presentations(
                                &[presentation],
                                &[manifest],
                                request_id,
                            )
                            .await
                        {
                            ctx.send(
                                reply,
                                Res::CredentialResponse(CredRes::VerifyCredentialPresentation(
                                    valid,
                                )),
                            )
                            .await
                        } else {
                            err()
                        }
                    }
                    AddCredential(profile_id, credential) => {
                        if let Ok(()) = self.profile(&profile_id).add_credential(credential) {
                            ctx.send(reply, Res::CredentialResponse(CredRes::AddCredential))
                                .await
                        } else {
                            err()
                        }
                    }
                    GetCredential(profile_id, credential) => {
                        if let Ok(c) = self.profile(&profile_id).get_credential(&credential) {
                            ctx.send(reply, Res::CredentialResponse(CredRes::GetCredential(c)))
                                .await
                        } else {
                            err()
                        }
                    }
                }
            }
        }
    }
}

impl EntityWorker {
    async fn add_profile_state(&mut self, profile_state: ProfileState) -> Result<()> {
        let id = profile_state.identifier().await.unwrap();
        self.profiles.insert(id, profile_state);
        Ok(())
    }

    fn remove_profile<I: Into<ProfileIdentifier>>(&mut self, profile_id: I) -> Result<()> {
        self.profiles
            .remove(&profile_id.into())
            .expect("remove_profile failed");
        Ok(())
    }
}
