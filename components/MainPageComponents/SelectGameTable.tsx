import {Location, Reservation} from "../../types/wrapper";
import React, {ReactElement, useEffect, useMemo} from "react";
import {SupabaseClient, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../../types/database.types";
import {useRouter} from "next/router";
import {useListState, useScrollIntoView} from "@mantine/hooks";
import useRestrictionsQuery from "../../hooks/useRestrictionsQuery";
import useGuestsQuery from "../../hooks/useGuestsQuery";
import {dateToISOString, isDateWeekend} from "../../utils/date";
import {ActionIcon, Group, Text} from "@mantine/core";
import {MdRefresh} from "react-icons/md";
import {RegistrationHours} from "./RegistrationHours";

function fetchReservations(
    supabase: SupabaseClient<Database>,
    setReservations: (data: Reservation[]) => void
) {
    supabase.from('rezervari')
        .select('*')
        .gte('start_date', dateToISOString(new Date))
        .eq('cancelled', false)
        .order('created_at', {ascending: true})
        .then(value => {
            if (value.data != null) {
                setReservations(value.data)
                console.log("Reservations updated")
            }
        })
}

export interface SelectGameTableProps {
    location: Location,
    selectedDateISO: string,
    selectedStartHour: number | null,
    onSetStartHour: (s: number) => void,
}

function SelectGameTable(
    {
        location,
        selectedDateISO,
        selectedStartHour,
        onSetStartHour
    }: SelectGameTableProps
): ReactElement {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()
    const [reservations, reservationsHandle] = useListState<Reservation>([])
    const {data: restrictions} = useRestrictionsQuery(new Date)
    const {data: guests} = useGuestsQuery(new Date)
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        fetchReservations(supabase, reservationsHandle.setState);

        const reservationListener = supabase.channel('rezervari')
            .on(
                'postgres_changes',
                {event: '*', schema: 'public', table: 'rezervari'},
                (payload) => {
                    if (payload.eventType == "INSERT") {
                        if (payload.new.cancelled === false) {
                            reservationsHandle.setState((prev) => {
                                    return [...prev, payload.new as Reservation]
                                }
                            )
                        }
                    } else if (payload.eventType == "UPDATE") {
                        fetchReservations(supabase, reservationsHandle.setState) // TODO Could make this more efficient
                    } else {
                        reservationsHandle.filter(value => value.id != payload.old.id)
                    }
                }
            )
            .subscribe()

        return () => {
            reservationListener?.unsubscribe()
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    useEffect(() => scrollIntoView({alignment: 'center'}), [scrollIntoView, selectedDateISO])

    const registrationHours = useMemo(() => {
        if (selectedDateISO == null) {
            return {start: 0, end: 0, duration: 0}
        }

        if (isDateWeekend(new Date(selectedDateISO))) {
            return {
                start: location.weekend_start_hour,
                end: location.weekend_end_hour,
                duration: location.weekend_reservation_duration,
            }
        }

        return {
            start: location.start_hour,
            end: location.end_hour,
            duration: location.reservation_duration,
        }
    }, [location, selectedDateISO])

    const selectedDateReservations =
        reservations.filter(value => value.start_date == selectedDateISO)
    const selectedDateInvites =
        guests?.filter(value => value.start_date == selectedDateISO) || []
    const selectedDateRestrictions =
        restrictions?.filter(value => value.date == selectedDateISO) || []

    return <>
        <Group position={'apart'} align={"center"}>
            <Group align={"center"} spacing={'xs'}>
                <Text weight={600} ref={targetRef}>Data selectată:</Text>
                <Text color={"blue"}>{new Date(selectedDateISO).toLocaleDateString('ro-RO')}</Text>
            </Group>

            <ActionIcon
                variant={'light'} radius={'xl'} size={36}
                onClick={() => router.reload()}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>

        {RegistrationHours(selectedDateReservations, selectedDateRestrictions, selectedDateInvites, selectedStartHour, onSetStartHour, registrationHours)}
    </>
}

export default React.memo(SelectGameTable)
