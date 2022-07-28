import {Models} from "appwrite";

export default interface Profile extends Models.Document {
    user_id: string;
    is_paid_member: boolean;
}
