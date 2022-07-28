import {Models} from "appwrite/src/models";
import {LocationName} from "./Room";

export default interface GameTable extends Models.Document {
    name: string;
    type: string;
    color: string;
    has_robot: boolean;
    location: LocationName
}
