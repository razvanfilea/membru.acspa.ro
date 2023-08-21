import {Button, Divider, Group, Popover, Stack, Text} from "@mantine/core";
import React, {ReactElement} from "react";
import {MdOutlineNoAccounts, MdVpnKey} from "react-icons/md";
import {GuestInvite, MemberTypes, Profile, Reservation, ReservationRestriction} from "../../types/wrapper";
import {Database} from "../../types/database.types";
import {useSupabaseClient} from "@supabase/auth-helpers-react";
import useProfileData from "../../hooks/useProfileData";
import useProfilesQuery from "../../hooks/useProfilesQuery";

function TableButton(
    startHour: number,
    selectedStartHour: number | null,
    onSetStartHour: (s: number) => void
) {
    return (
        <Button
            variant={(startHour == selectedStartHour) ? "filled" : "outline"}
            key={startHour}
            fullWidth={false}
            onClick={() => onSetStartHour(startHour)}>
            Rezervare
        </Button>
    )
}

interface IRegistrationHoursProps {
    start: number,
    end: number,
    duration: number,
}

export function RegistrationHours(
    selectedDateReservations: Reservation[],
    selectedRestrictions: ReservationRestriction[],
    selectedDateInvites: GuestInvite[],
    selectedStartHour: number | null,
    onSetStartHour: (s: number) => void,
    {start, end, duration}: IRegistrationHoursProps
) {
    const supabase = useSupabaseClient<Database>()
    const userProfile = useProfileData()

    const {data: profiles} = useProfilesQuery()

    let lastIndex = 0;
    let content: ReactElement[] = [];

    for (let hour = start; hour < end; hour += duration) {
        const restriction = selectedRestrictions.find(value => value.start_hour == hour)

        content.push(<Stack key={hour}>
            <Group noWrap={true} style={{marginLeft: "1em", marginRight: "1em"}} spacing={'lg'}>
                <Text>{`Ora ${hour} - ${hour + duration}`}:</Text>

                {!restriction ? (
                    TableButton(hour, selectedStartHour, onSetStartHour)
                ) : (
                    <Text color={'red'} size={'lg'}>{restriction.message}</Text>
                )}
            </Group>

            {!restriction &&
                <Group style={{marginLeft: "1em", marginRight: "1em"}} spacing={"xs"}>
                    <Text>Listă înscriși: </Text>
                    {selectedDateReservations.filter(value => value.start_hour == hour).map((reservation, index) => {
                        lastIndex = index;
                        const profile = profiles?.find(value => value.id == reservation.user_id)

                        if (!profile)
                            return <></>

                        const icon = profile.has_key ? <MdVpnKey/> : <></>;
                        const buttonColor = profile.role == MemberTypes.Antrenor ? 'orange' : (profile.has_key ? 'blue' : 'gray');

                        return <Popover width={200} withArrow={true} shadow={"md"} key={reservation.id}>
                            <Popover.Target>
                                <Button color={buttonColor} radius={'xl'}
                                        size={'xs'} rightIcon={icon}>{index + 1}. {profile.name}</Button>
                            </Popover.Target>

                            <Popover.Dropdown>
                                <Stack align={'center'}>
                                    <Text size="sm">Creat
                                        pe {new Date(reservation.created_at).toLocaleDateString("ro-RO")}</Text>

                                    {(userProfile.profile?.role === MemberTypes.Fondator || reservation.user_id === userProfile.profile?.id) &&

                                        <Button onClick={async () => {
                                            const newData: Reservation = {
                                                ...reservation,
                                                cancelled: true
                                            }

                                            await supabase.from('rezervari')
                                                .update(newData)
                                                .eq('id', reservation.id)
                                        }
                                        }>Anulează</Button>

                                    }
                                </Stack>
                            </Popover.Dropdown>
                        </Popover>

                    })}

                    {selectedDateInvites.filter(value => value.start_hour == hour).map((invite, index) => {
                        return <Button
                            key={invite.created_at} color={invite.special ? 'pink' : 'cyan'} radius={'xl'}
                            size={'xs'} rightIcon={<MdOutlineNoAccounts/>}>
                            {lastIndex + index + 2}. {invite.guest_name}
                        </Button>
                    })}
                </Group>
            }

            <Divider variant={"dashed"}/>
        </Stack>);
    }

    return content;
}
