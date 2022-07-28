import {Account, Avatars, Client, Databases} from "appwrite";

export const Server = {
    endpoint: process.env.REACT_APP_ENDPOINT,
    project: process.env.REACT_APP_PROJECT,
    databaseID: process.env.REACT_APP_DATABASE_ID,
};

const client = new Client()
    .setEndpoint(Server.endpoint)
    .setProject(Server.project);

const account = new Account(client);
const database = new Databases(client, Server.databaseID);
const avatars = new Avatars(client);

export const appwrite = {client, account, database, avatars};

export async function userIsLoggedIn(): Promise<boolean> {
    try {
        await appwrite.account.get()
        return true
    } catch (e) {
        return false
    }
}


