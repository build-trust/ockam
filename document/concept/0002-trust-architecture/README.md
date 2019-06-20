# Trust Architecture

We build connected systems so we can rely on the automation they bring to our lives and our businesses. Our wish is to trust these systems to collect information and take actions on our behalves.

This trust, however, should not be absolute. An overall system is typically composed of several moving parts (entities). To deliver the promise of the entire system, every entity within the system must establish trust in every interaction it has with any other entity that is part of the system. The operating environment is also usually highly dynamic – devices (entities) may enter or leave a deployment, firmware is updated frequently, configuration may be changed by an administrator, attackers may try to compromise the system etc.

Architects of such complex, dynamic systems need __simple protocols and tools that allow us to easily make explicit, nuanced decisions about trust__ within the granular context of each interaction at play within our systems. Broad stroke decisions can result in critical weaknesses.

Before we discuss further, let’s establish a shared understanding of what it means to trust. In the book _Achieving Digital Trust, Jeffrey Ritter_ writes:

> Trust (or the absence of trust) is the resulting sum of a __rules-based, information-fueled calculation__.

> Trust is the affirmative output of a disciplined, analytical decision process that __measures and scores the suitability of the next actions__ taken by you, your team, your business, or your community.

> Trust is the calculation of the __probability of outcomes__.

When deciding to act on any new information — a sensor reading message, a remote alarm message, a message that commands an actuator or contains a firmware update — we must apply a set of rules on all the information available to us, within that context, to decide if we can trust this new information enough to act on it.

Since connected systems are, at their core, [messaging systems](../0001-secure-connectivity-and-messaging) a natural place to apply these rules is when one entity (trustor) receives a new message from another entity (trustee) and is deciding on whether on not to act on this message.

By focusing attention on such application layer exchange of messages, Ockam hides the complexity of dealing with a variety of transport protocols, hardware and network topologies. This change in focus makes it simpler to __explicitly consider subtle factors that shape trust__ within the architecture of a connected IoT solution.

The question then becomes: [_what rules should the trustor apply in a particular situation?_](../0003-minimum-criteria-for-trust)
