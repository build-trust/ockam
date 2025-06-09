from ockam import Agent, Memory, Model, Node


async def main(node):
    docs = {
        "sec-330": "https://raw.githubusercontent.com/AlextheYounga/us-federal-code/refs/heads/master/usc/title-15-commerce-and-trade/chapter-9a-weather-modification-activities-or-attempts%3B-reporting-requirement/sec-330.md",
        "sec-330a": "https://raw.githubusercontent.com/AlextheYounga/us-federal-code/refs/heads/master/usc/title-15-commerce-and-trade/chapter-9a-weather-modification-activities-or-attempts%3B-reporting-requirement/sec-330a.md",
        "sec-330b": "https://raw.githubusercontent.com/AlextheYounga/us-federal-code/refs/heads/master/usc/title-15-commerce-and-trade/chapter-9a-weather-modification-activities-or-attempts%3B-reporting-requirement/sec-330b.md",
        "sec-330c": "https://raw.githubusercontent.com/AlextheYounga/us-federal-code/refs/heads/master/usc/title-15-commerce-and-trade/chapter-9a-weather-modification-activities-or-attempts%3B-reporting-requirement/sec-330c.md",
        "sec-330d": "https://raw.githubusercontent.com/AlextheYounga/us-federal-code/refs/heads/master/usc/title-15-commerce-and-trade/chapter-9a-weather-modification-activities-or-attempts%3B-reporting-requirement/sec-330d.md",
        "sec-330e": "https://raw.githubusercontent.com/AlextheYounga/us-federal-code/refs/heads/master/usc/title-15-commerce-and-trade/chapter-9a-weather-modification-activities-or-attempts%3B-reporting-requirement/sec-330e.md",
    }

    us_federal_code = StaticKnowledgeProvider("us_federal_code")
    for name, url in docs.items():
        await us_federal_code.add_document(name, url, content_type="text/markdown")

    await Agent.start(
        node=node,
        name="henry",
        instructions="You are Henry, an expert legal assistant",
        model=Model("nova-micro-v1"),
        knowledge=us_federal_code,
    )


Node.start(main)
