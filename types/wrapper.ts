import {Tables} from "./database.types";
import {isDateWeekend} from "../utils/date";

export const enum LocationName {
    Gara = "gara",
    Boromir = "boromir"
}

export type GlobalVars = Tables<'global_vars'>

export type Guest = Tables<'guests'>

export type Location = Tables<'locations'>

export type Profile = Tables<'profiles'>

export type Reservation = Tables<'rezervari'>

export type ReservationRestriction = Tables<'reservations_restrictions'>

export type FreeDay = Tables<'free_days'>

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
