use crate::cli_state::projects::Projects;
use ockam_core::compat::rand::random_string;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Route};
use ockam_multiaddr::proto::{DnsAddr, Project};
use ockam_multiaddr::{MultiAddr, ProtoValue, Protocol};
use ockam_node::Context;
use ockam_transport_tcp::{TcpInlet, TcpInletOptions};
use rand::prelude::SliceRandom;
use std::fmt::Display;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::timeout;

pub struct ReservedMultiAddr {
    pub original: MultiAddr,
    pub selected: MultiAddr,
    pub route_key: String,
    inlet: Arc<TcpInlet>,
    confirmed: bool,
}

impl ReservedMultiAddr {
    /// The connection was successfully instantiated, and the inlet was updated with the new route.
    /// Returns the original MultiAddr and the route_key.
    pub fn confirm(
        mut self,
        context: &Context,
        route: Route,
        options: TcpInletOptions,
    ) -> ockam_core::Result<(MultiAddr, String)> {
        self.inlet
            .update_outlet_route_and_unpause(context, &self.route_key, route, options)?;
        self.confirmed = true;
        Ok((self.original.clone(), self.route_key.clone()))
    }
}

impl Drop for ReservedMultiAddr {
    fn drop(&mut self) {
        if !self.confirmed {
            // If the connection fails, or other errors occur before being confirmed, the route is removed.
            self.inlet.remove_route(&self.selected.to_string());
        }
    }
}

const CACHE_DURATION: Duration = Duration::from_secs(20);

#[derive(Clone, Debug)]
struct Variant {
    address: MultiAddr,
    route_key: String,
}

#[derive(Clone, Debug)]
struct CacheEntry {
    original: MultiAddr,
    variants: Vec<Variant>,
}

/// Cache for the resolved MultiAddr variants
#[derive(Clone, Debug)]
struct VariantsCache {
    /// The list of original MultiAddr and the resolved MultiAddr variants
    entries: Vec<CacheEntry>,

    /// Keep a list of previous variants to preserve the route_key mappings.
    /// This is mainly for DNS records that may rotate over time.
    previous_variants: Vec<Variant>,

    /// The timestamp when the cache was created
    timestamp: std::time::Instant,
}

impl VariantsCache {
    /// Returns true if the cache is still valid and has at least one variant available.
    fn is_valid(&self) -> bool {
        // TODO: use the lowest DNS TTL as the cache duration
        self.timestamp.elapsed() <= CACHE_DURATION
            && self.entries.iter().any(|entry| !entry.variants.is_empty())
    }

    /// Returns the route_key of the provided MultiAddr, if it exists.
    /// The relative variant is also removed from the cache.
    fn take_variant_route_key(
        &mut self,
        original: &MultiAddr,
        variant: &MultiAddr,
    ) -> Option<String> {
        self.entries
            .iter_mut()
            .find(|entry| &entry.original == original)
            .and_then(|entry| {
                let index = entry.variants.iter().position(|v| &v.address == variant);
                index.map(|index| {
                    let variant = entry.variants.remove(index);
                    variant.route_key
                })
            })
            .or_else(|| {
                let index = self
                    .previous_variants
                    .iter()
                    .position(|v| &v.address == variant);
                index.map(|index| {
                    let variant = self.previous_variants.remove(index);
                    variant.route_key
                })
            })
    }

    /// Returns a list of every remaining (unused) MultiAddr variant with the relative route_key.
    /// This is to keep the mapping consistent in case DNS records rotate.
    pub(crate) fn take_unused_variants(self) -> Vec<Variant> {
        self.entries
            .into_iter()
            .flat_map(|entry| entry.variants)
            .chain(self.previous_variants)
            .collect()
    }

    /// Selects a random MultiAddr from the cache, excluding the provided route_keys.
    /// If all variants are exhausted, a random one is returned.
    fn select_one_random_except(&self, except: &[String]) -> Option<(&MultiAddr, Variant)> {
        let mut rng = rand::thread_rng();
        let variants: Vec<(&MultiAddr, &Variant)> = self
            .entries
            .iter()
            .flat_map(|entry| {
                entry
                    .variants
                    .iter()
                    .map(move |variant| (&entry.original, variant))
            })
            .filter(|(_original, variant)| !except.contains(&variant.route_key))
            .collect();

        if variants.is_empty() {
            // in case we exhausted all variants, we return a random one with a random route_key
            let view: Vec<(&MultiAddr, &Variant)> = self
                .entries
                .iter()
                .flat_map(|entry| {
                    entry
                        .variants
                        .iter()
                        .map(move |variant| (&entry.original, variant))
                })
                .collect();

            view.choose(&mut rng).copied().map(|(original, variant)| {
                (
                    original,
                    Variant {
                        address: variant.address.clone(),
                        route_key: random_string(),
                    },
                )
            })
        } else {
            variants
                .choose(&mut rng)
                .copied()
                .map(|(original, variant)| (original, variant.clone()))
        }
    }
}

#[derive(Clone)]
pub(in super::super) struct OutletMultiAddrSelector {
    pub(in super::super) outlet_addresses: Vec<MultiAddr>,
    cache: Arc<AsyncMutex<Option<VariantsCache>>>,
}

impl OutletMultiAddrSelector {
    pub fn new(outlet_addresses: Vec<MultiAddr>) -> Self {
        Self {
            outlet_addresses,
            cache: Default::default(),
        }
    }

    /// Creates a new [VariantsCache] complete with all variants from the provided outlet addresses.
    /// The previous cache must be provided, when present, to preserve the `route_key` mappings.
    async fn create_complete_cache(
        &self,
        projects: &Projects,
        mut previous_cache: Option<VariantsCache>,
    ) -> VariantsCache {
        let timestamp = std::time::Instant::now();
        let mut cache_entries: Vec<CacheEntry> = Vec::with_capacity(self.outlet_addresses.len());

        for original in &self.outlet_addresses {
            //TODO: authorize the project either via identity or via credentials
            let multiaddr = match Self::convert_project_multiaddr(projects, original).await {
                Ok(multiaddr) => multiaddr,
                Err(error) => {
                    warn!("Skipping MultiAddr {} due an error: {}", original, error);
                    continue;
                }
            };

            let multiaddrs = match Self::expand_dns_entries(&multiaddr).await {
                Ok(multiaddrs) => multiaddrs,
                Err(error) => {
                    warn!("Skipping MultiAddr {} due an error: {}", multiaddr, error);
                    continue;
                }
            };

            cache_entries.push(CacheEntry {
                original: original.clone(),
                variants: multiaddrs
                    .into_iter()
                    .map(|variant| Variant {
                        route_key: previous_cache
                            .as_mut()
                            .and_then(|c| c.take_variant_route_key(original, &variant))
                            .unwrap_or_else(random_string),
                        address: variant,
                    })
                    .collect(),
            });
        }

        let previous_variants = if let Some(previous_cache) = previous_cache {
            previous_cache.take_unused_variants()
        } else {
            Vec::new()
        };

        VariantsCache {
            entries: cache_entries,
            timestamp,
            previous_variants,
        }
    }

    /// Selects a MultiAddr from the list of outlet addresses and reserves the route to avoid
    /// concurrent connections to the same outlet.
    pub(super) async fn select(
        &self,
        context: &Context,
        inlet: &Arc<TcpInlet>,
        projects: &Projects,
    ) -> ockam_core::Result<ReservedMultiAddr> {
        let mut guard = self.cache.lock().await;
        let cache = {
            match &*guard {
                Some(cache) if cache.is_valid() => {}
                Some(_cache) => {
                    let previous = guard.take();
                    *guard = Some(self.create_complete_cache(projects, previous).await);
                }
                None => {
                    *guard = Some(self.create_complete_cache(projects, None).await);
                }
            };
            guard.as_ref().unwrap()
        };

        loop {
            let route_keys = inlet.list_all_route_keys();
            let (original, selected) = match cache.select_one_random_except(&route_keys) {
                Some(variant) => variant,
                None => {
                    let delay = Duration::from_secs(15);
                    warn!(
                        "No available MultiAddr found for inlet {}, retrying in {} seconds",
                        inlet,
                        delay.as_secs()
                    );
                    tokio::time::sleep(delay).await;
                    continue;
                }
            };

            if inlet.reserve_route_key(context, selected.route_key.clone())? {
                break Ok(ReservedMultiAddr {
                    inlet: inlet.clone(),
                    original: original.clone(),
                    selected: selected.address,
                    confirmed: false,
                    route_key: selected.route_key,
                });
            } else {
                trace!("Route already taken by another thread, retrying...");
            }
        }
    }

    /// Converts /project/<name> into its MultiAddr representation
    async fn convert_project_multiaddr(
        projects: &Projects,
        multiaddr: &MultiAddr,
    ) -> Result<MultiAddr, Error> {
        if let Some((dnsaddr, position)) = multiaddr.find(&[Project::CODE.into()]) {
            let project_name = dnsaddr.cast::<Project>().ok_or_else(|| {
                Error::new(
                    Origin::Channel,
                    Kind::Invalid,
                    "invalid project name in outlet address",
                )
            })?;
            let project = projects.get_project_by_name(&project_name).await?;

            let (before, _) = multiaddr.split(position);
            let (_, after) = multiaddr.split(position + 1);

            let mut combined = MultiAddr::new(before.registry().clone());

            // assuming the last two pieces are /service/api,
            // we need to convert them into /secure/api
            let mut project_multiaddr: Vec<ProtoValue> =
                project.project_multiaddr()?.iter().collect();
            if project_multiaddr.is_empty() {
                return Err(Error::new(
                    Origin::Channel,
                    Kind::Invalid,
                    "Project multiaddress doesn't contain a secure channel endpoint",
                ));
            }

            // drop /service/api
            project_multiaddr.truncate(project_multiaddr.len() - 1);

            combined.try_extend(before.iter())?;
            combined.try_extend(project_multiaddr)?;
            combined.try_extend("/secure/api".parse::<MultiAddr>()?.iter())?;
            combined.try_extend(after.iter())?;

            Ok(combined)
        } else {
            Ok(multiaddr.clone())
        }
    }

    /// Converts /dnsaddr/<host> into a list of MultiAddr with resolved IP addresses.
    /// Returns an error if resolution fails or empty
    async fn expand_dns_entries(multiaddr: &MultiAddr) -> Result<Vec<MultiAddr>, Error> {
        if let Some((dnsaddr, position)) = multiaddr.find(&[DnsAddr::CODE.into()]) {
            let host: &str = &dnsaddr.cast::<DnsAddr>().ok_or_else(|| {
                Error::new(
                    Origin::Channel,
                    Kind::Invalid,
                    "invalid DNS address in outlet address",
                )
            })?;

            let resolved: Vec<SocketAddr> = timeout(
                Duration::from_millis(1_000),
                // tokio does require the port to be present even when it is not used
                tokio::net::lookup_host(format!("{host}:0")),
            )
            .await
            .map_err(|_timeout| {
                Error::new(
                    Origin::Channel,
                    Kind::Invalid,
                    "Timeout while resolving DNS address".to_string(),
                )
            })?
            .map_err(|error| {
                Error::new(
                    Origin::Channel,
                    Kind::Invalid,
                    format!("Error while resolving DNS address: {error:?}"),
                )
            })?
            .collect();

            if resolved.is_empty() {
                return Err(Error::new(
                    Origin::Channel,
                    Kind::Invalid,
                    "DNS address resolved to an empty list",
                ));
            }

            let mut entries = Vec::with_capacity(resolved.len());

            for socket_address in resolved {
                let (before, _) = multiaddr.split(position);
                let (_, after) = multiaddr.split(position + 1);

                let mut combined = MultiAddr::new(before.registry().clone());
                let ip_multiaddr: MultiAddr = match socket_address {
                    SocketAddr::V4(address) => format!("/ip4/{}", address.ip()).parse()?,
                    SocketAddr::V6(address) => format!("/ip6/{}", address.ip()).parse()?,
                };
                combined.try_extend(before.iter())?;
                combined.try_extend(ip_multiaddr.into_iter())?;
                combined.try_extend(after.iter())?;

                entries.push(combined);
            }

            Ok(entries)
        } else {
            Ok(vec![multiaddr.clone()])
        }
    }
}

impl Display for OutletMultiAddrSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.outlet_addresses
                .iter()
                .map(|addr| addr.to_string())
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}

#[cfg(test)]
mod test {
    use super::OutletMultiAddrSelector;
    use super::*;
    use crate::orchestrator::project::models::ProjectModel;
    use crate::test_utils::start_manager_for_tests;

    #[ockam::test]
    async fn expand_project_and_dns(context: &mut Context) -> ockam::Result<()> {
        let handler = start_manager_for_tests(context, None, None).await?;

        let multiaddresses: Vec<MultiAddr> = vec![
            "/project/default/service/outlet".parse()?,
            "/secure/api/service/outlet".parse()?,
        ];

        let projects = handler.cli_state.projects();

        projects
            .store_project(
                crate::orchestrator::project::Project::import(ProjectModel {
                    id: "id".to_string(),
                    name: "default".to_string(),
                    space_name: "space".to_string(),
                    access_route: "/dnsaddr/orchestrator.ockam.io/tcp/1234/service/api".to_string(),
                    space_id: "space-id".to_string(),
                    ..Default::default()
                })
                .await?,
            )
            .await?;

        let cache = OutletMultiAddrSelector::new(multiaddresses.clone())
            .create_complete_cache(&projects, None)
            .await;

        assert_eq!(cache.entries.len(), 2);

        assert_eq!(
            cache.entries[0].original.to_string(),
            "/project/default/service/outlet"
        );
        assert_eq!(cache.entries[0].variants.len(), 2);
        let variants = &cache.entries[0].variants;

        let address = variants[0].address.to_string();
        assert!(address.starts_with("/ip4/"));
        assert!(address.ends_with("/tcp/1234/secure/api/service/outlet"));

        let address = variants[1].address.to_string();
        assert!(address.starts_with("/ip4/"));
        assert!(address.ends_with("/tcp/1234/secure/api/service/outlet"));

        assert_eq!(
            cache.entries[1].original.to_string(),
            "/secure/api/service/outlet"
        );
        assert_eq!(cache.entries[1].variants.len(), 1);
        assert_eq!(
            cache.entries[1].variants[0].address.to_string(),
            "/secure/api/service/outlet"
        );

        let mut previous_cache = cache;
        let mut cache = OutletMultiAddrSelector::new(multiaddresses)
            .create_complete_cache(&projects, Some(previous_cache.clone()))
            .await;

        // sort both cache and previous cache to compare them
        previous_cache.entries.iter_mut().for_each(|entry| {
            entry.variants.sort_by(|a, b| a.route_key.cmp(&b.route_key));
        });

        cache.entries.iter_mut().for_each(|entry| {
            entry.variants.sort_by(|a, b| a.route_key.cmp(&b.route_key));
        });

        // verify that the newer cache keeps the same route_keys
        assert_eq!(cache.entries[0].variants.len(), 2);
        let variants = &cache.entries[0].variants;
        let previous_variants = &previous_cache.entries[0].variants;
        assert_eq!(variants[0].route_key, previous_variants[0].route_key);
        assert_eq!(variants[1].route_key, previous_variants[1].route_key);

        assert_eq!(cache.entries[1].variants.len(), 1);
        assert_eq!(
            cache.entries[1].variants[0].route_key,
            previous_cache.entries[1].variants[0].route_key
        );

        Ok(())
    }

    #[ockam::test]
    async fn verify_unused_route_key_is_kept(context: &mut Context) -> ockam::Result<()> {
        let handler = start_manager_for_tests(context, None, None).await?;

        let projects = handler.cli_state.projects();

        let cache = OutletMultiAddrSelector::new(vec!["/secure/api/service/outlet1".parse()?])
            .create_complete_cache(&projects, None)
            .await;

        assert_eq!(cache.entries.len(), 1);
        let route_key1 = cache.entries[0].variants[0].route_key.clone();

        let cache = OutletMultiAddrSelector::new(vec!["/secure/api/service/outlet2".parse()?])
            .create_complete_cache(&projects, Some(cache))
            .await;

        let cache = OutletMultiAddrSelector::new(vec!["/secure/api/service/outlet1".parse()?])
            .create_complete_cache(&projects, Some(cache))
            .await;

        assert_eq!(route_key1, cache.entries[0].variants[0].route_key);

        Ok(())
    }
}
