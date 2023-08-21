import {Database} from "./database.types";

export const enum LocationName {
    Gara = "gara",
    Boromir = "boromir"
}

type Tables = Database['public']['Tables']

export type GuestInvite = Tables['guest_invites']['Row']

export type Location = Tables['locations']['Row']

export type Profile = Tables['profiles']['Row']

export type Reservation = Tables['rezervari']['Row']

export type ReservationRestriction = Tables['reservations_restrictions']['Row']

export function getStartTime(reservation: Reservation): Date {
    return new Date(reservation.start_date);
}

export function getEndDateDuration(date: Date, hours: number): Date {
    return new Date(date.getTime() + hours * 1000 * 60 * 60);
}

export function getEndTime(reservation: Reservation): Date {
    return getEndDateDuration(new Date(reservation.start_date), reservation.duration);
}

export const enum MemberTypes {
    Membru = "Membru",
    Cotizant1 = "Cotizant 1s",
    Cotizant2 = "Cotizant 2s",
    Antrenor = "Antrenor",
    Fondator = "Fondator",
    Invalid = "Invalid"
}
