import {Models} from "appwrite";
import {LocationName} from "./Room";

export interface BaseReservation {
    table_id: string;
    start_date: string;
    duration: number;
    user_id: string;
    location: LocationName;
    state?: ReservationState;
}

export interface Reservation extends BaseReservation, Models.Document {
}

export function getStartDate(reservation: BaseReservation): Date {
    return new Date(reservation.start_date);
}

export function getEndDateDuration(date: Date, hours: number): Date {
    return new Date(date.getTime() + hours * 1000 * 60 * 60);
}

export function getEndDate(reservation: BaseReservation): Date {
    return getEndDateDuration(new Date(reservation.start_date), reservation.duration);
}

export const enum ReservationState {
    PendingApproval = "pending",
    Approved = "approved",
    Canceled = "cancelled",
    Invalid = "invalid"
}

