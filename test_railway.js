const fetch = require('node-fetch');

const TOKEN = "df30c8d8-35b7-4c94-9f1c-a10040cb902f";
const URL = "https://backboard.railway.app/graphql/v2";

async function run() {
    const res = await fetch(URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            "Authorization": `Bearer ${TOKEN}`
        },
        body: JSON.stringify({
            query: `
                query {
                    me {
                        name
                        email
                    }
                    projects {
                        edges {
                            node {
                                id
                                name
                            }
                        }
                    }
                }
            `
        })
    });
    const data = await res.json();
    console.log(JSON.stringify(data, null, 2));
}

run().catch(console.error);
