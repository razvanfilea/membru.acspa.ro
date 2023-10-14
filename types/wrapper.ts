import {Database} from "./database.types";
import {isDateWeekend} from "../utils/date";

export const enum LocationName {
    Gara = "gara",
    Boromir = "boromir"
}

type Tables = Database['public']['Tables']

export type GlobalVars = Tables['global_vars']['Row']

export type Guest = Tables['guests']['Row']

export type Location = Tables['locations']['Row']

export type Profile = Tables['profiles']['Row']

export type Reservation = Tables['rezervari']['Row']

export type ReservationRestriction = Tables['reservations_restrictions']['Row']

export function getEndHour(reservation: Reservation, location: Location): number {
    const duration = isDateWeekend(new Date(reservation.start_date)) ? location.weekend_reservation_duration : location.reservation_duration;
    return reservation.start_hour + duration;
}

export const enum MemberTypes {
    Membru = "Membru",
    Cotizant1 = "Cotizant 1s",
    Cotizant2 = "Cotizant 2s",
    Antrenor = "Antrenor",
    Fondator = "Fondator",
    Invalid = "Invalid"
}
